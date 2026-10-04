//! Explicit, visible assembled checks. Only non-content receipts live in tmux.
use crate::workspace::Error;
use std::{
    collections::hash_map::DefaultHasher,
    fs::{self, File},
    hash::{Hash, Hasher},
    io::Write,
    os::unix::{ffi::OsStrExt, fs::MetadataExt, io::AsRawFd, process::CommandExt},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

fn invalid(s: &str) -> Error {
    Error::Invalid(s.into())
}
fn git(path: &Path, args: &[&str]) -> Result<Vec<u8>, Error> {
    let out = crate::workspace::checkout_git(path, args).output()?;
    if !out.status.success() {
        return Err(invalid("Verification checkout metadata unavailable"));
    }
    Ok(out.stdout)
}
fn safe_path(path: &Path) -> Result<(), Error> {
    let text = path
        .to_str()
        .ok_or_else(|| invalid("Verification checkout must be UTF-8"))?;
    if text.is_empty() || text.chars().any(char::is_control) {
        return Err(invalid(
            "Verification checkout paths must be nonempty and contain no control characters",
        ));
    }
    Ok(())
}
fn root(path: &Path) -> Result<PathBuf, Error> {
    safe_path(path)?;
    let bytes = git(path, &["rev-parse", "--show-toplevel"])?;
    let text =
        std::str::from_utf8(&bytes).map_err(|_| invalid("Verification checkout must be UTF-8"))?;
    // Strip Git's single record terminator, never characters belonging to a path.
    let text = text
        .strip_suffix('\n')
        .ok_or_else(|| invalid("Verification checkout metadata malformed"))?;
    let resolved = Path::new(text);
    safe_path(resolved)?;
    let canonical = resolved.canonicalize()?;
    safe_path(&canonical)?;
    Ok(canonical)
}

fn digest<T: Hash>(value: &T) -> String {
    let mut h = DefaultHasher::new();
    value.hash(&mut h);
    format!("{:016x}", h.finish())
}
fn key(path: &Path) -> String {
    format!("@drudwyn_verify_{}", digest(&path))
}
fn now() -> String {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format!("{}.{:09}", elapsed.as_secs(), elapsed.subsec_nanos())
}

fn encode(s: &str) -> String {
    crate::recovery::encode(Path::new(s))
}
fn decode(s: &str) -> Option<String> {
    crate::recovery::decode(s)?
        .into_os_string()
        .into_string()
        .ok()
}
fn lifetime(pid: u32) -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        // proc stat contains process metadata, never argv, environment or output.
        let stat = fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        let (_, fields) = stat.rsplit_once(") ")?;
        let fields: Vec<_> = fields.split_whitespace().collect();
        if fields.len() < 20 || matches!(fields[0], "Z" | "X") {
            return None;
        }
        Some(fields[19].into())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let out = Command::new("ps")
            .args(["-p", &pid.to_string(), "-o", "lstart=,stat="])
            .output()
            .ok()?;
        let text = String::from_utf8(out.stdout).ok()?;
        let fields: Vec<_> = text.split_whitespace().collect();
        (out.status.success() && fields.len() == 6 && !fields[5].starts_with(['Z', 'X']))
            .then(|| fields[..5].join("-"))
    }
}

fn valid_binding(value: &str) -> bool {
    let Some((pid, birth)) = value.split_once(':') else {
        return false;
    };
    if !pid.bytes().all(|b| b.is_ascii_digit()) || !pid.parse::<u32>().is_ok_and(|p| p > 0) {
        return false;
    }
    #[cfg(target_os = "linux")]
    {
        !birth.is_empty()
            && birth.bytes().all(|b| b.is_ascii_digit())
            && birth.parse::<u64>().is_ok_and(|ticks| ticks > 0)
    }
    #[cfg(not(target_os = "linux"))]
    {
        let f: Vec<_> = birth.split('-').collect();
        if f.len() != 5
            || !["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"].contains(&f[0])
            || ![
                "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
            ]
            .contains(&f[1])
            || !f[2].parse::<u8>().is_ok_and(|d| (1..=31).contains(&d))
            || !f[4].parse::<u32>().is_ok_and(|y| y > 0)
        {
            return false;
        }
        let time: Vec<_> = f[3].split(':').collect();
        time.len() == 3
            && time.iter().enumerate().all(|(i, part)| {
                part.len() == 2
                    && part.bytes().all(|b| b.is_ascii_digit())
                    && part
                        .parse::<u8>()
                        .is_ok_and(|n| n < if i == 0 { 24 } else { 60 })
            })
    }
}
fn timestamp(value: &str) -> Option<(u64, u32)> {
    let (seconds, nanos) = value.split_once('.')?;
    if seconds.is_empty()
        || !seconds.bytes().all(|b| b.is_ascii_digit())
        || nanos.len() != 9
        || !nanos.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    Some((seconds.parse().ok()?, nanos.parse().ok()?))
}

fn alive(binding: &str) -> bool {
    binding
        .split_once(':')
        .is_some_and(|(pid, birth)| pid.parse().ok().and_then(lifetime).as_deref() == Some(birth))
}
pub(crate) fn binding(pid: u32) -> Result<String, Error> {
    Ok(format!(
        "{pid}:{}",
        lifetime(pid).ok_or_else(|| invalid("Verification runner lifetime unavailable"))?
    ))
}
// Path and inode identify the actual checkout. Stat facts detect repeated edits
// even while porcelain remains dirty. No content hashes, diffs or file reads.
fn observation(path: &Path) -> Result<(String, String), Error> {
    let revision = String::from_utf8(git(path, &["rev-parse", "--verify", "HEAD^{commit}"])?)
        .map_err(|_| invalid("Verification revision unavailable"))?
        .trim()
        .to_owned();
    let mut h = DefaultHasher::new();
    let meta = fs::metadata(path)?;
    (meta.dev(), meta.ino()).hash(&mut h);
    git(
        path,
        &[
            "rev-parse",
            "--path-format=absolute",
            "--git-dir",
            "--git-common-dir",
        ],
    )?
    .hash(&mut h);
    git(path, &["symbolic-ref", "--quiet", "HEAD"])?.hash(&mut h);
    git(
        path,
        &["status", "--porcelain=v1", "-z", "--untracked-files=all"],
    )?
    .hash(&mut h);
    let names = git(
        path,
        &[
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ],
    )?;
    for name in names.split(|b| *b == 0).filter(|b| !b.is_empty()) {
        name.hash(&mut h);
        let file = path.join(std::ffi::OsStr::from_bytes(name));
        match fs::symlink_metadata(file) {
            Ok(m) => (
                m.dev(),
                m.ino(),
                m.mode(),
                m.len(),
                m.mtime(),
                m.mtime_nsec(),
                m.ctime(),
                m.ctime_nsec(),
            )
                .hash(&mut h),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => 0u8.hash(&mut h),
            Err(e) => return Err(e.into()),
        }
    }
    Ok((revision, format!("{:016x}", h.finish())))
}
#[derive(Clone)]
struct Receipt {
    path: String,
    check: String,
    revision: String,
    snapshot: String,
    start: String,
    end: String,
    exit: String,
    runner: String,
    child: String,
    stale: bool,
}
impl Receipt {
    fn serialize(&self) -> String {
        [
            "v1".into(),
            encode(&self.path),
            encode(&self.check),
            self.revision.clone(),
            self.snapshot.clone(),
            self.start.clone(),
            self.end.clone(),
            self.exit.clone(),
            self.runner.clone(),
            self.child.clone(),
            self.stale.to_string(),
        ]
        .join("|")
    }
    fn parse(s: &str) -> Option<Self> {
        let f: Vec<_> = s.trim_end().split('|').collect();
        if f.len() != 11 || f[0] != "v1" {
            return None;
        }
        let hex = |value: &str| value.bytes().all(|b| b.is_ascii_hexdigit());
        let known_exit = f[7].parse::<u8>().is_ok()
            || [
                "unknown · interrupted runner; no completion evidence",
                "unknown · command transmission failed",
                "unknown · interrupted by signal",
                "unavailable · check could not start",
            ]
            .contains(&f[7]);
        if ![40, 64].contains(&f[3].len())
            || !hex(f[3])
            || f[4].len() != 16
            || !hex(f[4])
            || timestamp(f[5]).is_none()
            || (!f[6].is_empty()
                && (timestamp(f[6]).is_none() || timestamp(f[6]) < timestamp(f[5])))
            || (f[6].is_empty() != f[7].is_empty())
            || (!f[7].is_empty() && !known_exit)
            || !matches!(f[10], "true" | "false")
            || !valid_binding(f[8])
            || (!f[9].is_empty() && !valid_binding(f[9]))
        {
            return None;
        }
        let check = decode(f[2])?;
        if check.is_empty() || check.len() > 160 || check.chars().any(char::is_control) {
            return None;
        }
        Some(Self {
            path: decode(f[1])?,
            check,
            revision: f[3].into(),
            snapshot: f[4].into(),
            start: f[5].into(),
            end: f[6].into(),
            exit: f[7].into(),
            runner: f[8].into(),
            child: f[9].into(),
            stale: f[10] == "true",
        })
    }
    fn display(&self, redact: bool) -> String {
        let state = if self.stale {
            "Stale · Not verified (observed checkout change)"
        } else if self.end.is_empty() {
            "Running"
        } else if self.child.is_empty() {
            "Not verified · check process lifetime unavailable"
        } else if self.exit == "0" {
            "Passed"
        } else {
            "Failed"
        };
        let label = |s: &str| {
            if redact {
                "[redacted]".to_owned()
            } else {
                format!("{s:?}")
            }
        };
        format!(
            "Assembled verification: {state}\nCheck: {}\nCheckout: {}\nTested revision: {}\nStarted: {}\nEnded: {}\nExit: {}\nReported worker checks: unknown (Review is not evidence)\nLimits: point-in-time; ignored files, external services and unobserved changes are not certified.\n",
            label(&self.check),
            label(&self.path),
            if redact { "[redacted]" } else { &self.revision },
            self.start,
            if self.end.is_empty() {
                "pending"
            } else {
                &self.end
            },
            if self.exit.is_empty() {
                "pending"
            } else {
                &self.exit
            }
        )
    }
}
fn read(path: &Path) -> Result<Option<Receipt>, Error> {
    let out = Command::new("tmux")
        .args(["show-option", "-sqv", &key(path)])
        .output()?;
    if !out.status.success() {
        return Err(invalid(
            "Live verification metadata unavailable; Not verified",
        ));
    }
    Ok(
        Receipt::parse(&String::from_utf8_lossy(&out.stdout))
            .filter(|r| Path::new(&r.path) == path),
    )
}
fn write(
    guard: &crate::lifecycle::LifecycleGuard,
    path: &Path,
    receipt: &Receipt,
) -> Result<(), Error> {
    guard
        .mutation(&["set-option", "-s", &key(path), &receipt.serialize()])
        .map_err(|_| invalid("Live verification receipt could not be saved"))
}
fn guard() -> Result<crate::lifecycle::LifecycleGuard, Error> {
    crate::lifecycle::LifecycleGuard::acquire()
        .map_err(|_| invalid("Verification metadata busy; retry"))
}
/// Inspection reconciles only existing receipts, and permanently records observed
/// drift. Loss of metadata cannot reconstruct success from Git or agent Review.
pub fn inspect(path: &Path, redact: bool) -> Result<String, Error> {
    let path = root(path)?;
    let lock = guard()?;
    let Some(mut r) = read(&path)? else {
        return Ok("Assembled verification: Not verified · missing live evidence\nReported worker checks: unknown (Review is not evidence)\n".into());
    };
    let before = r.serialize();
    drop(lock);
    match observation(&path) {
        Ok((revision, snapshot)) if revision == r.revision && snapshot == r.snapshot => {}
        _ => r.stale = true,
    }
    if r.end.is_empty() && !alive(&r.runner) {
        r.end = now();
        r.exit = "unknown · interrupted runner; no completion evidence".into();
    }
    let guard = guard()?;
    if read(&path)?.as_ref().map(Receipt::serialize).as_deref() != Some(&before) {
        return Err(invalid(
            "Verification evidence changed during inspection; refresh",
        ));
    }
    if before != r.serialize() {
        write(&guard, &path, &r)?;
    }
    let mut display = r.display(redact);
    if !r.end.is_empty() && alive(&r.child) {
        display.push_str("Previous check process remains alive; inspect it before rerun.\n");
    }
    Ok(display)
}
/// Run a user-selected shell program through transient stdin; inherit terminal
/// output directly. Commands never become argv, files, history or tmux options.
pub fn run(path: &Path, check: &str, command: &str, redact: bool) -> Result<(String, bool), Error> {
    run_with_progress(path, check, command, redact, |report| {
        println!("{report}");
        Ok(())
    })
}

/// The UI supplies presentation of the running receipt. Child output still goes
/// straight to its terminal and is never read, buffered, or retained here.
pub(crate) fn run_with_progress(
    path: &Path,
    check: &str,
    command: &str,
    redact: bool,
    started: impl FnOnce(&str) -> std::io::Result<()>,
) -> Result<(String, bool), Error> {
    if check.is_empty()
        || check.len() > 160
        || check.chars().any(char::is_control)
        || command.trim().is_empty()
        || command.len() > 65536
    {
        return Err(invalid(
            "Select a short check identity and a nonempty command (maximum 64 KiB)",
        ));
    }
    let path = root(path)?;
    let directory = File::open(&path)?;
    directory
        .try_lock()
        .map_err(|_| invalid("Checkout operation or check still running; retry later"))?;
    let (revision, snapshot) = observation(&path)?;
    let held = directory.metadata()?;
    let current = fs::metadata(&path)?;
    if held.dev() != current.dev() || held.ino() != current.ino() {
        return Err(invalid(
            "Verification checkout was replaced; inspect and select again",
        ));
    }
    let mut r = Receipt {
        path: path.to_string_lossy().into(),
        check: check.into(),
        revision,
        snapshot,
        start: now(),
        end: String::new(),
        exit: String::new(),
        runner: binding(std::process::id())?,
        child: String::new(),
        stale: false,
    };
    let lock = guard()?;
    if let Some(previous) = read(&path)? {
        if (previous.end.is_empty() && alive(&previous.runner)) || alive(&previous.child) {
            return Err(invalid(
                "Previous verification process still running; inspect before rerun",
            ));
        }
    }
    write(&lock, &path, &r)?;
    let mut shell = Command::new("bash");
    shell
        .args(["--noprofile", "--norc", "-s"])
        .current_dir(&path)
        .stdin(Stdio::piped())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    crate::workspace::isolate_git_namespace(&mut shell);
    for name in ["BASH_ENV", "ENV", "SHELLOPTS", "BASHOPTS", "HISTFILE"] {
        shell.env_remove(name);
    }
    // Carry the existing inode lock across exec separately from command stdin.
    // fcntl is async-signal-safe; only this child clears CLOEXEC on this descriptor.
    let fd = directory.as_raw_fd();
    unsafe {
        shell.pre_exec(move || {
            unsafe extern "C" {
                fn fcntl(fd: i32, cmd: i32, ...) -> i32;
                fn fchdir(fd: i32) -> i32;
            }
            const F_SETFD: i32 = 2;
            if fchdir(fd) < 0 || fcntl(fd, F_SETFD, 0) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    let mut child = match shell.spawn() {
        Ok(child) => child,
        Err(_) => {
            r.end = now();
            r.exit = "unavailable · check could not start".into();
            write(&lock, &path, &r)?;
            return Ok((r.display(redact), false));
        }
    };
    r.child = binding(child.id()).unwrap_or_default();
    write(&lock, &path, &r)?;
    drop(lock);
    started(&r.display(redact))?;
    let sent = child
        .stdin
        .take()
        .ok_or_else(|| invalid("Check input unavailable"))?
        .write_all(command.as_bytes());
    let status = child.wait()?;
    r.end = now();
    r.exit = if sent.is_err() {
        "unknown · command transmission failed".into()
    } else {
        status
            .code()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "unknown · interrupted by signal".into())
    };
    let changed = observation(&path)
        .map(|v| v != (r.revision.clone(), r.snapshot.clone()))
        .unwrap_or(true);
    let lock = guard()?;
    // Never recreate lost metadata or overwrite a receipt replaced during the run.
    let Some(current) = read(&path)? else {
        return Ok((
            "Assembled verification: Not verified · live receipt was lost\n".into(),
            false,
        ));
    };
    if current.runner != r.runner
        || current.start != r.start
        || current.child != r.child
        || current.revision != r.revision
        || current.snapshot != r.snapshot
        || current.check != r.check
        || !current.end.is_empty()
    {
        return Err(invalid("Verification receipt changed; result unavailable"));
    }
    r.stale = current.stale || changed;
    write(&lock, &path, &r)?;
    let passed = r.exit == "0" && !r.stale && !r.child.is_empty();
    Ok((r.display(redact), passed))
}
