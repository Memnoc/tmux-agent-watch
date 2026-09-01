use std::{
    collections::BTreeSet,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
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
    let repo = git(&request.repo, &["rev-parse", "--show-toplevel"])?;
    let repo = PathBuf::from(repo);
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
            .arg(&repo)
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
            let _ = Command::new("git")
                .arg("-C")
                .arg(&repo)
                .args(["worktree", "remove", "--force"])
                .arg(&target)
                .status();
            let _ = Command::new("git")
                .arg("-C")
                .arg(&repo)
                .args(["branch", "-D", &request.branch])
                .status();
            return Err(error);
        }
    };
    for (name, value) in [
        ("@agent_watch_branch", request.branch.as_str()),
        ("@agent_watch_worktree", target.to_str().unwrap_or("")),
        ("@agent_watch_repo", repo.to_str().unwrap_or("")),
        ("@agent_watch_git_status", "clean"),
        ("@agent_watch_message", ""),
    ] {
        tmux_ok(Command::new("tmux").args(["set-option", "-wq", "-t", &window, name, value]))?;
    }
    Ok(Started {
        path: target,
        window_id: window,
    })
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
    if !Command::new("git")
        .arg("-C")
        .arg(&primary)
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{base}"),
        ])
        .status()?
        .success()
    {
        return Err(Error::Invalid(format!("base branch {base} does not exist")));
    }
    if !Command::new("git")
        .arg("-C")
        .arg(&primary)
        .args(["merge-base", "--is-ancestor", &branch, base])
        .status()?
        .success()
    {
        return Err(Error::Invalid(format!(
            "{branch} is not merged into {base}"
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
        "#{window_id}␟#{pane_current_path}",
    ]))?;
    let windows = panes
        .lines()
        .filter_map(|line| {
            let (id, candidate) = line.split_once('␟')?;
            (Path::new(candidate) == worktree).then(|| id.to_owned())
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
