use std::{
    collections::BTreeSet,
    ffi::OsString,
    fs,
    io::{self, Write},
    os::unix::{
        ffi::{OsStrExt, OsStringExt},
        fs::MetadataExt,
    },
    path::{Path, PathBuf},
    process::{Command, Stdio},
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("{0}")]
    Invalid(String),
    #[error("Git operation failed: {0}")]
    Git(String),
    #[error("tmux operation failed: {0}")]
    Tmux(String),
    #[error("I/O failed: {0}")]
    Io(#[from] io::Error),
}

pub struct Start {
    pub repo: PathBuf,
    pub branch: String,
    pub name: Option<String>,
    pub start_point: String,
    pub batch: Option<String>,
    pub root: Option<PathBuf>,
    pub command: Vec<String>,
    pub task_file: Option<String>,
}

pub struct Started {
    pub path: PathBuf,
    pub window_id: String,
    pub pane_id: String,
}

/// Resolve a starting point using local refs only. Never inherit HEAD implicitly.
#[derive(Clone, Debug)]
pub struct StartPoint {
    pub reference: String,
    pub commit: String,
}

pub fn resolve_start_point(
    repo: &Path,
    base: &str,
    from_current: bool,
) -> Result<StartPoint, Error> {
    if !from_current {
        git(repo, &["check-ref-format", &format!("refs/heads/{base}")])?;
    }
    let candidates = if from_current {
        vec![git(repo, &["symbolic-ref", "--quiet", "HEAD"]).unwrap_or_else(|_| "HEAD".into())]
    } else {
        // Respect the configured branch's upstream, including non-origin remotes.
        let upstream = git(
            repo,
            &[
                "for-each-ref",
                "--format=%(upstream)",
                &format!("refs/heads/{base}"),
            ],
        )?;
        let mut refs = Vec::new();
        if !upstream.is_empty() {
            refs.push(upstream);
        }
        refs.push(format!("refs/remotes/origin/{base}"));
        refs.push(format!("refs/heads/{base}"));
        refs
    };
    for reference in candidates {
        if let Ok(commit) = git(
            repo,
            &[
                "rev-parse",
                "--verify",
                "--end-of-options",
                &format!("{reference}^{{commit}}"),
            ],
        ) {
            return Ok(StartPoint { reference, commit });
        }
    }
    Err(Error::Invalid(format!(
        "starting branch {base} is unavailable; fetch it, configure @drudwyn-base-branch, or choose to continue from the current branch"
    )))
}

pub fn start(request: Start) -> Result<Started, Error> {
    if !Command::new("git")
        .args(["check-ref-format", "--branch", &request.branch])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?
        .success()
    {
        return Err(Error::Invalid(format!(
            "invalid branch name: {}",
            request.branch
        )));
    }
    let source = PathBuf::from(git(&request.repo, &["rev-parse", "--show-toplevel"])?);
    let worktrees = git(&source, &["worktree", "list", "--porcelain"])?;
    let repo = PathBuf::from(
        worktrees
            .lines()
            .find_map(|line| line.strip_prefix("worktree "))
            .ok_or_else(|| Error::Git("primary worktree not found".into()))?,
    );
    if Command::new("git")
        .arg("-C")
        .arg(&repo)
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{}", request.branch),
        ])
        .status()?
        .success()
    {
        return Err(Error::Invalid(format!(
            "branch already exists: {}",
            request.branch
        )));
    }
    let root = request.root.unwrap_or_else(|| {
        repo.parent().unwrap_or(Path::new(".")).join(format!(
            "{}-worktrees",
            repo.file_name().unwrap_or_default().to_string_lossy()
        ))
    });
    let target = root.join(request.branch.replace('/', "-"));
    if target.exists() {
        return Err(Error::Invalid(format!(
            "worktree path already exists: {}",
            target.display()
        )));
    }
    let commit = git(
        &source,
        &[
            "rev-parse",
            "--verify",
            "--end-of-options",
            &format!("{}^{{commit}}", request.start_point),
        ],
    )?;
    if std::env::var_os("DRUDWYN_CLIENT").is_some() {
        crate::navigation::client()?;
    }
    let batch = request
        .batch
        .as_deref()
        .map(crate::batch::load)
        .transpose()?;
    if let Some(batch) = &batch {
        if batch.repository != crate::coordinator::repository(&source)?
            || batch.source.commit != commit
        {
            return Err(Error::Invalid(
                "Batch source or repository does not match launch".into(),
            ));
        }
    }
    let project = if let Some(batch) = &batch {
        Some(batch.project.clone())
    } else {
        crate::coordinator::launch_project(&source)?
    };
    let name = request.name.as_deref().unwrap_or(&request.branch);
    if name.trim().is_empty() || name.chars().count() > 64 || name.chars().any(char::is_control) {
        return Err(Error::Invalid(
            "worker name must be 1–64 characters and contain no control characters".into(),
        ));
    }
    if let Some(reference) = &request.task_file {
        validate_task_reference(reference)?;
        let entry = git(
            &source,
            &["--literal-pathspecs", "ls-tree", &commit, "--", reference],
        )?;
        if !(entry.starts_with("100644 blob ") || entry.starts_with("100755 blob ")) {
            return Err(Error::Invalid("Task file is not available at the pinned source; commit it and deliberately choose a new source, or paste instructions. Uncommitted planning files are not inherited".into()));
        }
    }
    fs::create_dir_all(&root)?;
    git_ok(
        Command::new("git")
            .arg("-C")
            .arg(&source)
            .args(["worktree", "add", "-q", "-b", &request.branch])
            .arg(&target)
            .arg(&commit),
    )?;
    let command = if request.command.is_empty() {
        vec!["codex".into()]
    } else {
        request.command
    };
    let expected_agent = command
        .first()
        .and_then(|c| crate::domain::AgentKind::from_command(c));
    let mut launch = Command::new("tmux");
    launch.arg("new-window");
    if let Some(project) = &project {
        launch.args(["-t", project]);
    }
    let window = launch
        .args([
            "-d",
            "-P",
            "-F",
            "#{window_id}␟#{pane_id}␟#{pane_pid}",
            "-n",
            &tmux_argument(name),
            "-c",
        ])
        .arg(tmux_argument(&target.to_string_lossy().replace('#', "##")))
        // A fixed setup shell enables exit retention before exec. Worker
        // arguments remain separate argv entries, never interpolated shell code.
        .args([
            "sh",
            "-c",
            "tmux set-option -p -t \"$TMUX_PANE\" remain-on-exit on; exec \"$@\"",
            "drudwyn-launch",
        ])
        .args(command.into_iter().map(|arg| tmux_argument(&arg)))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let child = match window {
        Ok(child) => child,
        Err(error) => {
            // The tmux executable never started, so no worker could use this
            // allocation. Even here, only remove an unchanged, clean checkout.
            if remove_unused_start(&repo, &target, &request.branch, &commit) {
                return Err(Error::Io(error));
            }
            return Err(retained_start_error(error, &target, &request.branch, None));
        }
    };
    // Once tmux starts, even an error can follow creation of a worker. Never
    // kill its window or remove its checkout on an uncertain command result.
    let output = child
        .wait_with_output()
        .map_err(|error| retained_start_error(error, &target, &request.branch, None))?;
    if !output.status.success() {
        return Err(retained_start_error(
            String::from_utf8_lossy(&output.stderr).trim(),
            &target,
            &request.branch,
            None,
        ));
    }
    let identity = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let fields: Vec<_> = identity.split('␟').collect();
    let stable = |value: &str, prefix| {
        value.starts_with(prefix)
            && value.len() > 1
            && value[1..].bytes().all(|b| b.is_ascii_digit())
    };
    if fields.len() != 3
        || !stable(fields[0], '@')
        || !stable(fields[1], '%')
        || fields[2].parse::<u32>().is_err()
    {
        return Err(retained_start_error(
            "tmux did not return a window/pane/process identity",
            &target,
            &request.branch,
            None,
        ));
    }
    let (window, launch_pane, launch_pid) = (
        fields[0].to_owned(),
        fields[1].to_owned(),
        fields[2].to_owned(),
    );
    let initialize = || -> Result<(), Error> {
        // Bind startup before any slower project or Git initialization. The
        // launch wrapper retains even a process that exits before this point.
        set_window(&window, "@drudwyn_launch_pane", &launch_pane)?;
        set_window(&window, "@drudwyn_launch_pid", &launch_pid)?;
        crate::lifecycle::starting(&launch_pane, &launch_pid, expected_agent)
            .map_err(|error| Error::Invalid(error.to_string()))?;
        crate::coordinator::protect_name(&window)?;
        // A fast process may emit a rename escape before new-window returns.
        // Finish initialization with the requested name after blocking escapes;
        // no refresh or scan rewrites subsequent deliberate user renames.
        tmux_ok(Command::new("tmux").args([
            "rename-window",
            "-t",
            &window,
            "--",
            &tmux_argument(name),
        ]))?;
        if let Some(project) = &project {
            tmux_ok(Command::new("tmux").args([
                "set-option",
                "-wq",
                "-t",
                &window,
                "@drudwyn_project",
                project,
            ]))?;
        }
        if let Some(batch) = &batch {
            crate::batch::select_created(&batch.id, &window, &target)?;
        }
        for (name, value) in [
            ("@drudwyn_branch", request.branch.as_str()),
            ("@drudwyn_worktree", target.to_str().unwrap_or("")),
            ("@drudwyn_repo", repo.to_str().unwrap_or("")),
            ("@drudwyn_git_status", "clean"),
            ("@drudwyn_message", ""),
        ] {
            tmux_ok(Command::new("tmux").args(["set-option", "-wq", "-t", &window, name, value]))?;
        }
        if let Some(reference) = &request.task_file {
            validate_task_file(&target, reference)?;
            // The selected path is operational metadata; never read file content.
            set_window(&window, "@drudwyn_task_file", &tmux_argument(reference))?;
        }
        // tmux can report a new window before its command has had a chance to
        // exit. Do not publish a workspace until the initial process survives
        // a short startup frame and the target is still addressable.
        thread::sleep(Duration::from_millis(150));
        let live = tmux(Command::new("tmux").args([
            "display-message",
            "-p",
            "-t",
            &launch_pane,
            "#{window_id}\t#{pane_dead}\t#{pane_dead_status}",
        ]))?;
        let mut fields = live.split('\t');
        if fields.next() != Some(window.as_str()) {
            return Err(Error::Tmux("agent window exited during startup".into()));
        }
        if fields.next() != Some("0") {
            let status = fields
                .next()
                .filter(|value| !value.is_empty())
                .unwrap_or("unknown");
            return Err(Error::Tmux(format!(
                "agent exited during startup (exit status {status}); exit is not task completion"
            )));
        }
        let pane = delivery_target(&launch_pane)?;
        if pane.window != window || pane.pid != launch_pid {
            return Err(Error::Invalid(
                "Launch pane/process changed during initialization".into(),
            ));
        }
        set_window(&window, "@drudwyn_launch_pane", &launch_pane)?;
        set_window(&window, "@drudwyn_launch_pid", &launch_pid)?;
        // tmux output escapes some literal path characters. Keep the exact
        // checkout bytes for the delivery guard, rather than guessing how to
        // unescape a display label or falling back to a different directory.
        let checkout = target.canonicalize()?;
        let checkout = checkout
            .as_os_str()
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        set_window(&window, "@drudwyn_launch_checkout", &checkout)?;
        set_window(
            &window,
            "@drudwyn_launch_command",
            expected_agent.map(|a| a.command()).unwrap_or(&pane.command),
        )?;
        set_window(&window, "@drudwyn_delivery", "not_sent")?;
        set_window(&window, "@drudwyn_launch_stage", "observed")?;
        crate::lifecycle::scan().map_err(|error| Error::Invalid(error.to_string()))?;
        Ok(())
    };
    if let Err(error) = initialize() {
        let _ = crate::lifecycle::scan();
        return Err(retained_start_error(
            error,
            &target,
            &request.branch,
            Some(&window),
        ));
    }
    Ok(Started {
        path: target,
        window_id: window,
        pane_id: launch_pane,
    })
}

// tmux parses trailing semicolons as separators even with an argv API.
fn tmux_argument(value: &str) -> String {
    value
        .strip_suffix(';')
        .map(|p| format!("{p}\\;"))
        .unwrap_or_else(|| value.into())
}
fn set_window(window: &str, option: &str, value: &str) -> Result<(), Error> {
    tmux_ok(Command::new("tmux").args(["set-option", "-wq", "-t", window, option, value]))
}
fn validate_task_reference(reference: &str) -> Result<(), Error> {
    if reference.is_empty()
        || reference.chars().any(char::is_control)
        || Path::new(reference)
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err(Error::Invalid("Task file must be a repository-relative file path without parent components or controls".into()));
    }
    Ok(())
}
pub fn validate_task_file(checkout: &Path, reference: &str) -> Result<(), Error> {
    validate_task_reference(reference)?;
    let root = checkout.canonicalize()?;
    let path = checkout.join(reference).canonicalize().map_err(|_| {
        Error::Invalid("Task file is unavailable in the worker checkout; task not sent".into())
    })?;
    if !path.starts_with(root) || !path.is_file() {
        return Err(Error::Invalid(
            "Task file must be a file inside the worker checkout; task not sent".into(),
        ));
    }
    Ok(())
}
#[derive(PartialEq, Eq)]
struct DeliveryTarget {
    window: String,
    pane: String,
    pid: String,
    command: String,
    checkout: String,
}
fn delivery_target(target: &str) -> Result<DeliveryTarget, Error> {
    let record = tmux(Command::new("tmux").args([
        "display-message",
        "-p",
        "-t",
        target,
        "#{window_id}␟#{pane_id}␟#{pane_pid}␟#{pane_current_command}␟#{pane_dead}␟#{@drudwyn_launch_checkout}",
    ]))?;
    let fields: Vec<_> = record.split('␟').collect();
    if fields.len() != 6 || fields[4] != "0" {
        return Err(Error::Invalid(
            "Delivery target unavailable or exited; task not sent".into(),
        ));
    }
    Ok(DeliveryTarget {
        window: fields[0].into(),
        pane: fields[1].into(),
        pid: fields[2].into(),
        command: fields[3].into(),
        checkout: fields[5].into(),
    })
}

fn retained_start_error(
    error: impl std::fmt::Display,
    target: &Path,
    branch: &str,
    window: Option<&str>,
) -> Error {
    let window = window
        .map(|id| format!("; inspect window {id} if it still exists"))
        .unwrap_or_default();
    Error::Invalid(format!(
        "launch failed: {error}; retained worktree {} on branch {branch}{window}. Inspect the retained checkout before retrying; use a new branch/path for a separate worker",
        target.display()
    ))
}

fn remove_unused_start(repo: &Path, target: &Path, branch: &str, commit: &str) -> bool {
    if git(target, &["rev-parse", "HEAD"]).ok().as_deref() != Some(commit)
        || git(
            target,
            &[
                "status",
                "--porcelain",
                "--untracked-files=all",
                "--ignored",
            ],
        )
        .ok()
        .as_deref()
            != Some("")
    {
        return false;
    }
    if git_ok(
        Command::new("git")
            .arg("-C")
            .arg(repo)
            .args(["worktree", "remove"])
            .arg(target),
    )
    .is_err()
    {
        return false;
    }
    // Compare-and-delete avoids deleting a branch whose tip changed meanwhile.
    git(
        repo,
        &["update-ref", "-d", &format!("refs/heads/{branch}"), commit],
    )
    .is_ok()
}

/// Bounded startup observation uses process metadata only. A matching executable
/// is not proof of readiness/acceptance; the resulting receipt says only sent.
pub fn send_started(
    started: &Started,
    task: &str,
    file: bool,
    agent: Option<crate::domain::AgentKind>,
) -> Result<(), Error> {
    let mut ready = agent.is_none();
    for _ in 0..30 {
        let pane = bound_target(&started.pane_id)?;
        if agent.is_none() || crate::domain::AgentKind::from_command(&pane.command) == agent {
            ready = true;
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    if !ready {
        return Err(Error::Invalid("Worker created; startup delayed or process unrecognized; task NOT sent. Inspect the pane and deliberately deliver when ready".into()));
    }
    if file {
        validate_task_file(&started.path, task)?;
        deliver(
            &started.pane_id,
            &format!("Read the repository task file {task:?} and carry out its instructions."),
            false,
        )
    } else {
        deliver_task(&started.pane_id, task)
    }
}

/// Text is held only in this call and a uniquely named, delete-on-paste buffer.
/// A successful send is not evidence of agent acceptance or implementation.
pub fn deliver_task(target: &str, task: &str) -> Result<(), Error> {
    deliver(target, task, false)
}

pub fn deliver_reference(target: &str, reference: &str, retry: bool) -> Result<(), Error> {
    let pane = bound_target(target)?;
    let checkout = tmux(Command::new("tmux").args([
        "show-option",
        "-wqv",
        "-t",
        &pane.window,
        "@drudwyn_worktree",
    ]))?;
    validate_task_file(Path::new(&checkout), reference)?;
    set_window(
        &pane.window,
        "@drudwyn_task_file",
        &tmux_argument(reference),
    )?;
    deliver(
        &pane.pane,
        &format!("Read the repository task file {reference:?} and carry out its instructions."),
        retry,
    )
}

fn bound_target(target: &str) -> Result<DeliveryTarget, Error> {
    let selected = delivery_target(target)?;
    let pane = tmux(Command::new("tmux").args([
        "show-option",
        "-wqv",
        "-t",
        &selected.window,
        "@drudwyn_launch_pane",
    ]))?;
    if pane.is_empty() {
        return Ok(selected);
    }
    let bound = delivery_target(&pane)?;
    let pid = tmux(Command::new("tmux").args([
        "show-option",
        "-wqv",
        "-t",
        &selected.window,
        "@drudwyn_launch_pid",
    ]))?;
    if bound.window != selected.window
        || bound.pid != pid
        || (target.starts_with('%') && target != pane)
    {
        return Err(Error::Invalid("Launch pane/process changed; task not sent. Inspect the worker before deliberate recovery".into()));
    }
    Ok(bound)
}

fn same_process(expected: &DeliveryTarget) -> Result<(), Error> {
    let current = bound_target(&expected.pane)?;
    if current != *expected {
        return Err(Error::Invalid("Delivery pane/process changed; delivery uncertain. Inspect the worker before deliberate recovery".into()));
    }
    Ok(())
}

/// Lock an existing checkout inode, never a created lock file or task registry.
/// Sibling launches have distinct checkouts. Windows sharing a checkout also
/// serialize conservatively. The descriptor is close-on-exec; drop or process
/// death releases the kernel lock, including after an interrupted delivery.
fn delivery_guard(pane: &DeliveryTarget) -> Result<fs::File, Error> {
    if pane.checkout.is_empty()
        || pane.checkout.len() % 2 != 0
        || !pane.checkout.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(Error::Invalid(
            "Launch checkout identity missing or malformed; cannot guard delivery. Task not sent; no fallback attempted".into(),
        ));
    }
    let bytes = (0..pane.checkout.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&pane.checkout[i..i + 2], 16).unwrap())
        .collect();
    let checkout = PathBuf::from(OsString::from_vec(bytes));
    if !checkout.is_absolute() {
        return Err(Error::Invalid(
            "Launch checkout identity is not absolute; task not sent; no fallback attempted".into(),
        ));
    }
    let unavailable = |error| {
        Error::Invalid(format!(
            "Cannot guard delivery for the recorded worker checkout: {error}. Task not sent; no fallback attempted"
        ))
    };
    let directory = fs::File::open(&checkout).map_err(unavailable)?;
    let held = directory.metadata().map_err(unavailable)?;
    if !held.is_dir() {
        return Err(Error::Invalid(
            "Recorded worker checkout is not a directory; task not sent".into(),
        ));
    }
    match directory.try_lock() {
        Ok(()) => {}
        Err(fs::TryLockError::WouldBlock) => {
            return Err(Error::Invalid(
                "A delivery is already in progress for this worker checkout. Nothing resent; inspect it before deliberate retry".into(),
            ));
        }
        Err(fs::TryLockError::Error(error)) => return Err(unavailable(error)),
    }
    same_process(pane)?;
    let current = fs::metadata(&checkout).map_err(unavailable)?;
    if held.dev() != current.dev() || held.ino() != current.ino() {
        return Err(Error::Invalid(
            "Worker checkout changed while guarding delivery; task not sent".into(),
        ));
    }
    Ok(directory)
}

struct TransientBuffer(String);
impl Drop for TransientBuffer {
    fn drop(&mut self) {
        let _ = Command::new("tmux")
            .args(["delete-buffer", "-b", &self.0])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

pub fn deliver(target: &str, task: &str, retry: bool) -> Result<(), Error> {
    if task.trim().is_empty() {
        return Err(Error::Invalid("Task is empty; not sent".into()));
    }
    let pane = bound_target(target)?;
    // Hold through state checks, transmission, receipt, and buffer cleanup.
    let _guard = delivery_guard(&pane)?;
    let previous = tmux(Command::new("tmux").args([
        "show-option",
        "-wqv",
        "-t",
        &pane.window,
        "@drudwyn_delivery",
    ]))?;
    if !retry && !previous.is_empty() && previous != "not_sent" {
        return Err(Error::Invalid("A delivery was already attempted; inspect the agent before an explicit --retry. Nothing resent".into()));
    }
    let command = tmux(Command::new("tmux").args([
        "show-option",
        "-wqv",
        "-t",
        &pane.window,
        "@drudwyn_launch_command",
    ]))?;
    if !command.is_empty() && pane.command != command {
        return Err(Error::Invalid("Launch process changed or startup is delayed; task not sent. Inspect before deliberate recovery".into()));
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let buffer = TransientBuffer(format!("drudwyn-task-{}-{nonce}", std::process::id()));
    let mut child = Command::new("tmux")
        .args(["load-buffer", "-b", &buffer.0, "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    let written = child
        .stdin
        .take()
        .ok_or_else(|| Error::Invalid("task input unavailable".into()))?
        .write_all(task.as_bytes());
    let loaded = child.wait()?;
    if written.is_err() || !loaded.success() {
        return Err(Error::Tmux(
            "Could not load transient task; task not sent; worker retained".into(),
        ));
    }
    same_process(&pane)?;
    // Mark uncertainty before the first potentially transmitting operation. No
    // failure or subsequent invocation may silently resend this task.
    set_window(&pane.window, "@drudwyn_delivery", "uncertain")?;
    let transmit = || -> Result<(), Error> {
        tmux_ok(Command::new("tmux").args([
            "paste-buffer",
            "-d",
            "-p",
            "-r",
            "-b",
            &buffer.0,
            "-t",
            &pane.pane,
        ]))?;
        thread::sleep(Duration::from_millis(750));
        same_process(&pane)?;
        tmux_ok(Command::new("tmux").args(["send-keys", "-t", &pane.pane, "Enter"]))?;
        set_window(&pane.window, "@drudwyn_delivery", "sent")?;
        Ok(())
    };
    transmit().map_err(|_| Error::Invalid(format!("Worker created; task delivery uncertain; retained window {} pane {}. Inspect it before deliberate retry; nothing automatically resent", pane.window, pane.pane)))
}

pub fn finish(path: &Path, base: &str, yes: bool) -> Result<PathBuf, Error> {
    let worktree = PathBuf::from(git(path, &["rev-parse", "--show-toplevel"])?);
    if git(
        &worktree,
        &["rev-parse", "--path-format=absolute", "--git-dir"],
    )? == git(
        &worktree,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )? {
        return Err(Error::Invalid(
            "the primary checkout cannot be finished".into(),
        ));
    }
    let branch = git(&worktree, &["branch", "--show-current"])?;
    if branch.is_empty() {
        return Err(Error::Invalid(
            "detached worktrees must be handled manually".into(),
        ));
    }
    if !git(&worktree, &["status", "--porcelain"])?.is_empty() {
        return Err(Error::Invalid(
            "worktree is dirty; review, commit, or discard changes first".into(),
        ));
    }
    let list = git(&worktree, &["worktree", "list", "--porcelain"])?;
    let primary = PathBuf::from(
        list.lines()
            .find_map(|line| line.strip_prefix("worktree "))
            .ok_or_else(|| Error::Invalid("primary worktree not found".into()))?,
    );
    let branch_ref = format!("refs/heads/{branch}");
    let base_ref = format!("refs/heads/{base}");
    if !Command::new("git")
        .arg("-C")
        .arg(&primary)
        .args(["show-ref", "--verify", "--quiet", &base_ref])
        .status()?
        .success()
    {
        return Err(Error::Invalid(format!("base branch {base} does not exist")));
    }
    let merged_into_base = Command::new("git")
        .arg("-C")
        .arg(&primary)
        .args(["merge-base", "--is-ancestor", &branch_ref, &base_ref])
        .status()?
        .success();
    let contained_by_primary = Command::new("git")
        .arg("-C")
        .arg(&primary)
        .args(["merge-base", "--is-ancestor", &branch_ref, "HEAD"])
        .status()?
        .success();
    if !merged_into_base && !contained_by_primary {
        return Err(Error::Invalid(format!(
            "{branch} is not merged into {base} or the primary checkout"
        )));
    }
    if !yes {
        eprint!("Remove linked worktree for {branch}? The branch will be retained. [y/N] ");
        io::stderr().flush()?;
        let mut answer = String::new();
        io::stdin().read_line(&mut answer)?;
        if !matches!(answer.trim(), "y" | "Y" | "yes" | "YES") {
            return Err(Error::Invalid("cancelled".into()));
        }
    }
    let panes = tmux(Command::new("tmux").args([
        "list-panes",
        "-a",
        "-F",
        "#{window_id}␟#{pane_current_path}␟#{@drudwyn_worktree}",
    ]))?;
    let windows = panes
        .lines()
        .filter_map(|line| {
            let mut fields = line.split('␟');
            let id = fields.next()?;
            let current_path = Path::new(fields.next()?);
            let recorded_worktree = Path::new(fields.next()?);
            (current_path.starts_with(&worktree) || recorded_worktree == worktree)
                .then(|| id.to_owned())
        })
        .collect::<BTreeSet<_>>();
    git_ok(
        Command::new("git")
            .arg("-C")
            .arg(&primary)
            .args(["worktree", "remove"])
            .arg(&worktree),
    )?;
    for window in windows {
        let _ = Command::new("tmux")
            .args(["kill-window", "-t", &window])
            .status();
    }
    Ok(worktree)
}

pub(crate) fn git(path: &Path, args: &[&str]) -> Result<String, Error> {
    let mut c = Command::new("git");
    c.arg("-C").arg(path).args(args);
    let o = c.output()?;
    if o.status.success() {
        Ok(String::from_utf8_lossy(&o.stdout).trim().into())
    } else {
        Err(Error::Git(String::from_utf8_lossy(&o.stderr).trim().into()))
    }
}
fn git_ok(c: &mut Command) -> Result<(), Error> {
    let o = c.output()?;
    if o.status.success() {
        Ok(())
    } else {
        Err(Error::Git(String::from_utf8_lossy(&o.stderr).trim().into()))
    }
}
fn tmux(c: &mut Command) -> Result<String, Error> {
    let o = c.output()?;
    if o.status.success() {
        Ok(String::from_utf8_lossy(&o.stdout).trim().into())
    } else {
        Err(Error::Tmux(
            String::from_utf8_lossy(&o.stderr).trim().into(),
        ))
    }
}
fn tmux_ok(c: &mut Command) -> Result<(), Error> {
    tmux(c).map(|_| ())
}
