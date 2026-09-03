use std::{
    collections::BTreeSet,
    fs,
    io::{self, Write},
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
    pub root: Option<PathBuf>,
    pub command: Vec<String>,
}

pub struct Started {
    pub path: PathBuf,
    pub window_id: String,
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
    fs::create_dir_all(&root)?;
    git_ok(
        Command::new("git")
            .arg("-C")
            .arg(&source)
            .args(["worktree", "add", "-q", "-b", &request.branch])
            .arg(&target),
    )?;
    let command = if request.command.is_empty() {
        vec!["codex".into()]
    } else {
        request.command
    };
    let slug = request.branch.replace('/', "-");
    let window = tmux(
        Command::new("tmux")
            .args([
                "new-window",
                "-d",
                "-P",
                "-F",
                "#{window_id}",
                "-n",
                &slug,
                "-c",
            ])
            .arg(&target)
            .args(command),
    );
    let window = match window {
        Ok(value) => value,
        Err(error) => {
            rollback_start(&repo, &target, &request.branch, None);
            return Err(error);
        }
    };
    let initialize = || -> Result<(), Error> {
        for (name, value) in [
            ("@agent_watch_branch", request.branch.as_str()),
            ("@agent_watch_worktree", target.to_str().unwrap_or("")),
            ("@agent_watch_repo", repo.to_str().unwrap_or("")),
            ("@agent_watch_git_status", "clean"),
            ("@agent_watch_message", ""),
        ] {
            tmux_ok(Command::new("tmux").args(["set-option", "-wq", "-t", &window, name, value]))?;
        }
        // tmux can report a new window before its command has had a chance to
        // exit. Do not publish a workspace until the initial process survives
        // a short startup frame and the target is still addressable.
        thread::sleep(Duration::from_millis(150));
        let live = tmux(Command::new("tmux").args([
            "display-message",
            "-p",
            "-t",
            &window,
            "#{window_id}",
        ]))?;
        if live != window {
            return Err(Error::Tmux("agent window exited during startup".into()));
        }
        Ok(())
    };
    if let Err(error) = initialize() {
        rollback_start(&repo, &target, &request.branch, Some(&window));
        return Err(error);
    }
    Ok(Started {
        path: target,
        window_id: window,
    })
}

fn rollback_start(repo: &Path, target: &Path, branch: &str, window: Option<&str>) {
    if let Some(window) = window {
        let _ = Command::new("tmux")
            .args(["kill-window", "-t", window])
            .status();
    }
    let _ = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["worktree", "remove", "--force"])
        .arg(target)
        .status();
    let _ = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["branch", "-D", branch])
        .status();
}

pub fn deliver_task(window_id: &str, task: &str) -> Result<(), Error> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let buffer = format!("agent-watch-task-{}-{nonce}", std::process::id());
    let mut child = Command::new("tmux")
        .args(["load-buffer", "-b", &buffer, "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()?;
    child
        .stdin
        .take()
        .ok_or_else(|| Error::Invalid("task input unavailable".into()))?
        .write_all(task.as_bytes())?;
    if !child.wait()?.success() {
        return Err(Error::Tmux("could not load transient task".into()));
    }
    let result =
        tmux_ok(Command::new("tmux").args(["paste-buffer", "-d", "-b", &buffer, "-t", window_id]));
    if result.is_err() {
        let _ = Command::new("tmux")
            .args(["delete-buffer", "-b", &buffer])
            .status();
        return result;
    }
    // Interactive TUIs may process a bracketed paste asynchronously. Give the
    // editor one frame to settle before submitting, otherwise Enter can be
    // consumed while the pasted text remains in the input field.
    thread::sleep(Duration::from_millis(750));
    tmux_ok(Command::new("tmux").args(["send-keys", "-t", window_id, "Enter"]))
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
        "#{window_id}␟#{pane_current_path}␟#{@agent_watch_worktree}",
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

fn git(path: &Path, args: &[&str]) -> Result<String, Error> {
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
