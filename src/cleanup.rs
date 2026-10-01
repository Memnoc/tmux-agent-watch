//! Explicit branch-preserving worktree removal. Live process metadata is a
//! conservative veto, never a claim inferred from agent activity or completion.
use crate::{
    integration::{self, Checkout, Destination, Request},
    navigation::tmux,
    workspace::Error,
};
use std::{
    collections::{BTreeMap, BTreeSet, hash_map::DefaultHasher},
    fs::{self, File},
    hash::{Hash, Hasher},
    os::unix::{fs::MetadataExt, io::AsRawFd, process::CommandExt},
    path::PathBuf,
    process::{Command, Stdio},
};

fn invalid(s: &str) -> Error {
    Error::Invalid(s.into())
}
#[derive(Clone, Debug)]
pub struct Preview {
    pub request: Request,
    pub source: Checkout,
    pub destination: String,
    pub commit: String,
    pub token: String,
    target: Option<Checkout>,
}
impl Preview {
    pub fn display(&self, redact: bool) -> String {
        let label = |s: &str| {
            if redact {
                "[redacted]".to_owned()
            } else {
                s.to_owned()
            }
        };
        format!(
            "FINISH PREVIEW\nSource: {}\nSource commit: {}\nWorktree: {}\nChosen destination: {}\nDestination commit: {}\nClean and contained; no observed active writer or Git operation. Process observation is scoped; confirm no external writer uses this checkout.\nConfirm removal; the branch is retained. No push or deployment.\nReview token: {}\n",
            label(&self.source.reference),
            label(&self.source.commit),
            label(&self.source.path.to_string_lossy()),
            label(&self.destination),
            label(&self.commit),
            self.token
        )
    }
}
pub fn preview(request: Request) -> Result<Preview, Error> {
    let source = integration::checkout(&request.source)?;
    if source.git_dir == source.common {
        return Err(invalid("The primary checkout cannot be finished"));
    }
    integration::ready(&source)?;
    if !integration::raw(&source.path, &["ls-files", "--others", "-z"])?.is_empty() {
        return Err(invalid(
            "Untracked files (including ignored files) remain; preserve them before Finish",
        ));
    }
    let (branch, recorded) = match &request.destination {
        Destination::Branch(branch) => (branch.clone(), None),
        Destination::Batch(id) => {
            let batch = crate::batch::load(id)?;
            if integration::canonical(std::path::Path::new(&batch.repository))? != source.common {
                return Err(invalid(
                    "Batch belongs to another repository; choose the destination explicitly",
                ));
            }
            (batch.destination, Some(batch.checkout))
        }
    };
    let destination = format!("refs/heads/{branch}");
    integration::git(&source.path, &["check-ref-format", &destination])?;
    if destination == source.reference {
        return Err(invalid("Finish destination must be another branch"));
    }
    let commit = integration::git(
        &source.path,
        &[
            "rev-parse",
            "--verify",
            &format!("{destination}^{{commit}}"),
        ],
    )?;
    if !integration::ancestor(&source.path, &source.commit, &commit)? {
        return Err(invalid(
            "Worker is not contained in the chosen destination; integrate it there before Finish",
        ));
    }
    let records = integration::git(&source.path, &["worktree", "list", "--porcelain", "-z"])?;
    let checked_out = records
        .split('\0')
        .any(|f| f == format!("branch {destination}"));
    let target = if checked_out || recorded.is_some() {
        let target = integration::destination(&source.path, &branch, recorded.as_deref())?;
        if target.common != source.common
            || target.commit != commit
            || integration::operation(&target)?
        {
            return Err(invalid(
                "Destination changed or has an existing Git operation or lock; inspect before Finish",
            ));
        }
        Some(target)
    } else {
        None
    };
    writers(&source, &panes()?)?;
    let mut hash = DefaultHasher::new();
    (
        "finish-v1",
        &request.destination,
        &source,
        &destination,
        &commit,
        &target,
    )
        .hash(&mut hash);
    Ok(Preview {
        request,
        source,
        destination,
        commit,
        token: format!("{:016x}", hash.finish()),
        target,
    })
}

#[derive(Clone, PartialEq, Eq)]
struct Pane {
    window: String,
    id: String,
    pid: String,
    dead: bool,
    birth: Option<String>,
    path: Option<PathBuf>,
    cwd_identity: Option<(u64, u64)>,
    recorded: Vec<PathBuf>,
}
fn panes() -> Result<Vec<Pane>, Error> {
    let rows = tmux(&[
        "list-panes",
        "-a",
        "-F",
        "#{window_id}␟#{pane_id}␟#{pane_pid}␟#{pane_dead}␟#{pane_current_path}␟#{@drudwyn_launch_checkout}␟#{@drudwyn_recovery_checkout}",
    ])?;
    let mut result = BTreeMap::new();
    for row in rows.lines() {
        let f: Vec<_> = row.split('␟').collect();
        if f.len() != 7 {
            return Err(invalid("Pane ownership unavailable; no cleanup attempted"));
        }
        let proc = format!("/proc/{}/cwd", f[2]);
        let dead = f[3] == "1";
        let path = if dead {
            crate::recovery::selected_checkout(f[0], f[1]).ok()
        } else if cfg!(target_os = "linux") {
            fs::read_link(&proc).ok()
        } else {
            Some(PathBuf::from(f[4]))
        };
        let cwd_identity = if dead {
            None
        } else {
            let metadata = if cfg!(target_os = "linux") {
                fs::metadata(&proc).ok()
            } else {
                path.as_ref().and_then(|p| fs::metadata(p).ok())
            };
            metadata.map(|m| (m.dev(), m.ino()))
        };
        result.insert(
            f[1].to_owned(),
            Pane {
                window: f[0].into(),
                id: f[1].into(),
                pid: f[2].into(),
                dead,
                birth: if dead {
                    None
                } else {
                    f[2].parse()
                        .ok()
                        .and_then(|pid| crate::verification::binding(pid).ok())
                },
                path,
                cwd_identity,
                // Prior operational identity may veto deletion, but never
                // authorize window closure after a process replacement.
                recorded: f[5..7]
                    .iter()
                    .filter_map(|p| crate::recovery::decode(p))
                    .collect(),
            },
        );
    }
    Ok(result.into_values().collect())
}
struct Process {
    uid: String,
    pid: String,
    parent: String,
    shell: bool,
    exited: bool,
}
fn processes() -> Result<BTreeMap<String, Process>, Error> {
    let output = Command::new("ps")
        .args(["-eo", "uid=,pid=,ppid=,stat=,comm="])
        .output()?;
    if !output.status.success() {
        return Err(invalid(
            "Process observation unavailable; retain the worktree",
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let f: Vec<_> = line.split_whitespace().collect();
            (f.len() >= 5).then(|| {
                let name = std::path::Path::new(f[4])
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                let p = Process {
                    uid: f[0].into(),
                    pid: f[1].into(),
                    parent: f[2].into(),
                    shell: matches!(name, "bash" | "zsh" | "sh" | "dash" | "fish" | "ksh"),
                    exited: f[3].starts_with(['Z', 'X']),
                };
                (p.pid.clone(), p)
            })
        })
        .collect())
}
fn portable_cwds(uid: &str) -> Result<BTreeMap<String, PathBuf>, Error> {
    // lsof is a metadata-only fallback for hosts without /proc. Restrict it to
    // same-user cwd records; no argv, open-file content or terminal data.
    let output = Command::new("lsof")
        .args(["-a", "-u", uid, "-d", "cwd", "-F0pn"])
        .output()
        .map_err(|_| {
            invalid("Finish requires process cwd metadata (lsof unavailable); retain the worktree")
        })?;
    if !output.status.success() {
        return Err(invalid(
            "Process cwd observation unavailable; retain the worktree",
        ));
    }
    let mut paths = BTreeMap::new();
    let mut pid = None;
    for field in output.stdout.split(|b| *b == 0) {
        let field = field.strip_prefix(b"\n").unwrap_or(field);
        match field.first() {
            Some(b'p') => pid = std::str::from_utf8(&field[1..]).ok().map(str::to_owned),
            Some(b'n') => {
                if let (Some(pid), Ok(path)) = (&pid, std::str::from_utf8(&field[1..])) {
                    paths.insert(pid.clone(), PathBuf::from(path));
                }
            }
            _ => {}
        }
    }
    Ok(paths)
}
fn writers(source: &Checkout, panes: &[Pane]) -> Result<(), Error> {
    let all = processes()?;
    let self_pid = std::process::id().to_string();
    let me = all
        .get(&self_pid)
        .ok_or_else(|| invalid("Caller process unavailable; retain the worktree"))?;
    let mut caller = BTreeSet::from([self_pid.clone()]);
    let mut id = me.parent.as_str();
    // Exempt invocation shells only. An agent/editor/background ancestor is
    // still a potential writer, even if it invoked this command itself.
    for _ in 0..256 {
        let Some(p) = all.get(id) else { break };
        if !p.shell || !caller.insert(p.pid.clone()) {
            break;
        }
        id = &p.parent;
    }
    let mut owned = BTreeSet::new();
    for p in panes.iter().filter(|p| !p.dead) {
        let current = p.path.as_ref().is_some_and(|p| p.starts_with(&source.path));
        let recorded = crate::recovery::selected_checkout(&p.window, &p.id)
            .ok()
            .is_some_and(|p| p.starts_with(&source.path));
        if current || recorded || p.recorded.iter().any(|p| p.starts_with(&source.path)) {
            owned.insert(p.pid.clone());
        }
    }
    let portable = if cfg!(target_os = "linux") {
        BTreeMap::new()
    } else {
        portable_cwds(&me.uid)?
    };
    // The synchronously invoking shell may be outside tmux. Its background
    // descendants still belong to this checkout even after changing directories.
    for pid in caller.iter().filter(|pid| *pid != &self_pid) {
        let cwd = if cfg!(target_os = "linux") {
            fs::read_link(format!("/proc/{pid}/cwd")).ok()
        } else {
            portable.get(pid).cloned()
        };
        if cwd.is_some_and(|p| p.starts_with(&source.path)) {
            owned.insert(pid.clone());
        }
    }
    let mut unreadable = Vec::new();
    for p in all
        .values()
        .filter(|p| !p.exited && !caller.contains(&p.pid))
    {
        // Our completed metadata probes (notably ps itself) can appear in its
        // snapshot after they have been reaped. Absence is not an active writer.
        if cfg!(target_os = "linux")
            && matches!(fs::metadata(format!("/proc/{}",p.pid)),Err(e) if e.kind()==std::io::ErrorKind::NotFound)
        {
            continue;
        }
        let mut id = p.pid.as_str();
        let mut in_pane = false;
        for _ in 0..256 {
            if owned.contains(id) {
                in_pane = true;
                break;
            }
            match all.get(id) {
                Some(p) if p.parent != id => id = &p.parent,
                _ => break,
            }
        }
        let cwd = if cfg!(target_os = "linux") {
            fs::read_link(format!("/proc/{}/cwd", p.pid)).ok()
        } else {
            portable.get(&p.pid).cloned()
        };
        if cwd.is_none() && p.uid == me.uid && in_pane {
            unreadable.push(p.pid.clone());
        }
        let in_checkout = cwd.is_some_and(|path| path.starts_with(&source.path));
        if !in_pane && !in_checkout {
            continue;
        }
        return Err(Error::Invalid(format!(
            "An active writer or other live process (PID {}) may use this worktree; explicitly stop agents and other worker shells before Finish (Review is not proof)",
            p.pid
        )));
    }
    if !unreadable.is_empty() {
        let current = processes()?;
        if unreadable
            .iter()
            .any(|pid| current.get(pid).is_some_and(|p| !p.exited))
        {
            return Err(invalid(
                "Associated live process cwd unavailable; cannot establish active writer absence",
            ));
        }
    }
    if panes
        .iter()
        .any(|p| !p.dead && owned.contains(&p.pid) && (p.path.is_none() || p.birth.is_none()))
    {
        return Err(invalid(
            "Live pane working directory/lifetime unavailable; cannot establish active writer absence",
        ));
    }
    Ok(())
}
fn closable(source: &Checkout, panes: &[Pane]) -> Result<BTreeSet<String>, Error> {
    let coordinators = tmux(&["list-sessions", "-F", "#{@drudwyn_coordinator}"])?;
    let memberships = tmux(&["list-windows", "-a", "-F", "#{window_id}␟#{session_id}"])?;
    let mut sessions: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for row in memberships.lines() {
        if let Some((w, s)) = row.split_once('␟') {
            sessions.entry(s).or_default().insert(w);
        }
    }
    Ok(panes
        .iter()
        .filter(|p| {
            p.path
                .as_ref()
                .is_some_and(|path| path.starts_with(&source.path))
        })
        .map(|p| p.window.clone())
        .filter(|w| {
            !coordinators.lines().any(|c| c == w)
                && !panes.iter().any(|p| {
                    p.window == *w && std::env::var("TMUX_PANE").ok().as_deref() == Some(&p.id)
                })
                && panes.iter().filter(|p| p.window == *w).all(|p| {
                    p.path
                        .as_ref()
                        .is_some_and(|path| path.starts_with(&source.path))
                })
                && !sessions
                    .values()
                    .any(|windows| windows.contains(w.as_str()) && windows.len() == 1)
        })
        .collect())
}
pub fn apply(reviewed: &Preview) -> Result<PathBuf, Error> {
    let source = File::open(&reviewed.source.path)?;
    source.try_lock().map_err(|_|invalid("Worktree integration/recovery/delivery/verification already in progress; retry after it stops"))?;
    let m = source.metadata()?;
    if (m.dev(), m.ino()) != (reviewed.source.device, reviewed.source.inode) {
        return Err(invalid("Source changed after preview; review again"));
    }
    let guard_path = reviewed
        .target
        .as_ref()
        .map(|t| &t.path)
        .unwrap_or(&reviewed.source.common);
    let target = File::open(guard_path)?;
    target
        .try_lock()
        .map_err(|_| invalid("Destination action already in progress; retry after it stops"))?;
    let fresh = preview(reviewed.request.clone())?;
    if reviewed.token != fresh.token {
        return Err(invalid(
            "Source or destination changed after preview; review again",
        ));
    }
    let before = panes()?;
    writers(&fresh.source, &before)?;
    let windows = closable(&fresh.source, &before)?;
    // Final Git snapshot follows process/window observation, before any mutation.
    if preview(reviewed.request.clone())?.token != reviewed.token {
        return Err(invalid(
            "Source or destination changed after preview; review again",
        ));
    }
    let mut command = crate::workspace::checkout_git(&fresh.source.common, &["worktree", "remove"]);
    command
        .arg(&fresh.source.path)
        .stdin(Stdio::from(source.try_clone()?))
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let fd = target.as_raw_fd();
    unsafe {
        command.pre_exec(move || {
            unsafe extern "C" {
                fn fcntl(fd: i32, cmd: i32, ...) -> i32;
            }
            if fcntl(fd, 2, 0) < 0 {
                return Err(std::io::Error::last_os_error());
            }
            Ok(())
        });
    }
    if !command.status()?.success() {
        return Err(invalid(
            "Git worktree removal failed; branch and windows retained. Inspect Git state before retry; no force removal attempted",
        ));
    }
    // Revalidate pane identities, cwd inodes and coordinator/session roles after
    // removal. A renamed/replaced/mixed window is preserved, never broadly killed.
    let closure_guard=crate::lifecycle::LifecycleGuard::acquire()
        .map_err(|_|invalid("Worktree removed; window closure unavailable. Branch retained; inspect remaining windows"))?;
    let after = panes().map_err(|_| {
        invalid(
            "Worktree removed; branch retained; pane observation failed, inspect remaining windows",
        )
    })?;
    let mut normalized = after.clone();
    for p in &mut normalized {
        if let Some(old) = before.iter().find(|old| {
            old.id == p.id
                && old.pid == p.pid
                && old.dead == p.dead
                && old.birth == p.birth
                && old.cwd_identity == p.cwd_identity
        }) {
            if old.dead || p.cwd_identity.is_some() {
                p.path = old.path.clone();
            }
        }
    }
    let eligible = closable(&fresh.source, &normalized)
        .map_err(|_|invalid("Worktree removed; branch retained; window ownership unavailable, inspect remaining windows"))?;
    'windows: for window in windows.intersection(&eligible) {
        let old: Vec<_> = before.iter().filter(|p| p.window == *window).collect();
        let current: Vec<_> = normalized.iter().filter(|p| p.window == *window).collect();
        if old == current {
            // tmux evaluates this and queues the removal on its server thread.
            // A new/replaced pane between observation and dispatch vetoes closure.
            let mut condition = format!("#{{==:#{{window_panes}},{}}}", current.len());
            for p in &current {
                let member = format!(
                    "#{{m:*;{}/{}/{};*,;#{{P:#{{pane_id}}/#{{pane_pid}}/#{{pane_dead}};}}}}",
                    p.id,
                    p.pid,
                    if p.dead { 1 } else { 0 }
                );
                condition = format!("#{{&&:{condition},{member}}}");
            }
            for p in &current {
                let command = match tmux(&[
                    "display-message",
                    "-p",
                    "-t",
                    &p.id,
                    "#{pane_current_command}",
                ]) {
                    Ok(value) => value,
                    Err(_) => continue 'windows,
                };
                if !p.dead
                    && !matches!(
                        command.as_str(),
                        "bash" | "zsh" | "sh" | "dash" | "fish" | "ksh"
                    )
                {
                    continue 'windows;
                }
                let observed = format!(
                    "#{{P:#{{?#{{==:#{{pane_id}},{}}},#{{pane_current_command}},}}}}",
                    p.id
                );
                if command
                    .chars()
                    .any(|c| c.is_control() || matches!(c, ',' | '#' | '{' | '}' | '\\'))
                {
                    continue 'windows;
                }
                condition = format!("#{{&&:{condition},#{{==:{observed},{command}}}}}");
                // tmux's display output quotes shell metacharacters; never
                // unescape it into authority. Linux cwd is lossless kernel
                // metadata and compares to tmux's internal (unquoted) value.
                if !p.dead && cfg!(target_os = "linux") {
                    let Some(cwd) = after
                        .iter()
                        .find(|a| a.id == p.id)
                        .and_then(|p| p.path.as_ref())
                        .and_then(|p| p.to_str())
                    else {
                        continue 'windows;
                    };
                    if cwd
                        .chars()
                        .any(|c| c.is_control() || matches!(c, ',' | '#' | '{' | '}' | '\\'))
                    {
                        continue 'windows;
                    }
                    let observed = format!(
                        "#{{P:#{{?#{{==:#{{pane_id}},{}}},#{{pane_current_path}},}}}}",
                        p.id
                    );
                    condition = format!("#{{&&:{condition},#{{==:{observed},{cwd}}}}}");
                }
            }
            let coordinator = format!("#{{m:*;{window};*,;#{{S:#{{@drudwyn_coordinator}};}}}}");
            let singleton = format!(
                "#{{m:*;{window};*,;#{{S:#{{?#{{==:#{{session_windows}},1}},#{{W:#{{window_id}};}},}}}}}}"
            );
            condition = format!(
                "#{{&&:{condition},#{{&&:#{{==:{coordinator},0}},#{{==:{singleton},0}}}}}}"
            );
            let _ = closure_guard.mutation(&[
                "if-shell",
                "-F",
                "-t",
                window,
                &condition,
                &format!("kill-window -t {window}"),
            ]);
        }
    }
    Ok(fresh.source.path)
}
