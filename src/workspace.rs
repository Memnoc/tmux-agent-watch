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
    pub name: Option<String>,
    pub start_point: String,
    pub batch: Option<String>,
    pub root: Option<PathBuf>,
    pub command: Vec<String>,
}

pub struct Started {
    pub path: PathBuf,
    pub window_id: String,
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
    if name.trim().is_empty() || name.chars().any(char::is_control) {
        return Err(Error::Invalid(
            "worker name must be nonempty and contain no control characters".into(),
        ));
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
    let mut launch = Command::new("tmux");
    launch.arg("new-window");
    if let Some(project) = &project {
        launch.args(["-t", project]);
    }
    let window = launch
        .args(["-d", "-P", "-F", "#{window_id}", "-n", name, "-c"])
        .arg(&target)
        .args(command)
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
    let window = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if !window.starts_with('@')
        || window[1..].is_empty()
        || !window[1..].bytes().all(|c| c.is_ascii_digit())
    {
        return Err(retained_start_error(
            "tmux did not return a window identity",
            &target,
            &request.branch,
            None,
        ));
    }
    let initialize = || -> Result<(), Error> {
        crate::coordinator::protect_name(&window)?;
        // A fast process may emit a rename escape before new-window returns.
        // Finish initialization with the requested name after blocking escapes;
        // no refresh or scan rewrites subsequent deliberate user renames.
        tmux_ok(Command::new("tmux").args(["rename-window", "-t", &window, "--", name]))?;
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
            crate::batch::select(&batch.id, &window)?;
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
        // tmux can report a new window before its command has had a chance to
        // exit. Do not publish a workspace until the initial process survives
        // a short startup frame and the target is still addressable.
        thread::sleep(Duration::from_millis(150));
        let live = tmux(Command::new("tmux").args([
            "display-message",
            "-p",
            "-t",
            &window,
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
        Ok(())
    };
    if let Err(error) = initialize() {
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

pub fn deliver_task(window_id: &str, task: &str) -> Result<(), Error> {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let buffer = format!("drudwyn-task-{}-{nonce}", std::process::id());
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
