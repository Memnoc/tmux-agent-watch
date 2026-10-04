//! Git owns surviving checkouts; tmux owns their optional live terminals.
use crate::{navigation::tmux, workspace::Error};
use std::{
    collections::BTreeSet,
    ffi::OsString,
    fs,
    os::unix::ffi::{OsStrExt, OsStringExt},
    os::unix::fs::MetadataExt,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Debug)]
pub struct Checkout {
    pub path: PathBuf,
    pub branch: String,
    pub unavailable: Option<String>,
    pub live: Vec<String>,
    pub stopped: Vec<String>,
    pub task_file: Option<String>,
}
impl Checkout {
    pub fn state(&self) -> &str {
        self.unavailable
            .as_deref()
            .unwrap_or(if self.live.is_empty() {
                if self.stopped.is_empty() {
                    "Recoverable"
                } else {
                    "Stopped workspace"
                }
            } else {
                "Live workspace"
            })
    }
    pub fn display(&self, redact: bool) -> String {
        format!(
            "{} | {} | {} | task reference: {} | historical exit/checks: Unknown",
            self.state(),
            if redact {
                "[redacted]".into()
            } else {
                self.branch.clone()
            },
            if redact {
                "[redacted]".into()
            } else {
                self.path.display().to_string()
            },
            if redact {
                "[redacted]"
            } else {
                self.task_file
                    .as_deref()
                    .unwrap_or("Unknown; select explicitly")
            }
        )
    }
}

/// Resolve the selected stable pane using known operational identity or native
/// process cwd metadata. Display labels are never unescaped into authority.
pub(crate) fn selected_checkout(window: &str, pane: &str) -> Result<PathBuf, Error> {
    let format = "#{window_id}␟#{pane_id}␟#{pane_pid}␟#{pane_dead}␟#{@drudwyn_launch_pane}␟#{@drudwyn_launch_pid}␟#{@drudwyn_launch_checkout}␟#{@drudwyn_recovery_checkout}␟#{pane_current_path}";
    let before = tmux(&["display-message", "-p", "-t", pane, format])?;
    let f: Vec<_> = before.split('␟').collect();
    if f.len() != 9 || f[0] != window || f[1] != pane {
        return Err(Error::Invalid(
            "Selected pane changed; refresh before recovery".into(),
        ));
    }
    let encoded = if f[4] == pane && f[5] == f[2] {
        f[6]
    } else {
        f[7]
    };
    let path = if !encoded.is_empty() {
        let path = decode(encoded)
            .filter(|path| path.is_absolute())
            .ok_or_else(|| {
                Error::Invalid(
                    "Selected checkout identity is malformed; refresh or reselect the checkout"
                        .into(),
                )
            })?;
        path
    } else {
        if f[3] != "0" {
            return Err(Error::Invalid(
                "Selected pane exited without a known checkout; explicitly select a repository"
                    .into(),
            ));
        }
        fs::read_link(format!("/proc/{}/cwd", f[2])).unwrap_or_else(|_| PathBuf::from(f[8]))
    };
    if !path.is_absolute() {
        return Err(Error::Invalid(
            "Selected pane has no absolute checkout identity; explicitly select a repository"
                .into(),
        ));
    }
    let path = path.canonicalize().map_err(|_| {
        Error::Invalid(
            "Selected pane checkout is unavailable; no display-path unescaping attempted".into(),
        )
    })?;
    if tmux(&["display-message", "-p", "-t", pane, format])? != before {
        return Err(Error::Invalid(
            "Selected pane changed; refresh before recovery".into(),
        ));
    }
    Ok(path)
}

pub fn list(repo: &Path) -> Result<Vec<Checkout>, Error> {
    // Enumerate only the selected repository, never search the filesystem.
    let output = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(["worktree", "list", "--porcelain", "-z"])
        .output()?;
    if !output.status.success() {
        return Err(Error::Git(
            String::from_utf8_lossy(&output.stderr).trim().into(),
        ));
    }
    let data = String::from_utf8(output.stdout)
        .map_err(|_| Error::Invalid("Worktree paths must be valid UTF-8".into()))?;
    let panes = tmux(&[
        "list-panes",
        "-a",
        "-F",
        "#{window_id}␟#{pane_pid}␟#{pane_dead}␟#{pane_current_path}␟#{@drudwyn_worktree}␟#{@drudwyn_task_file}␟#{@drudwyn_launch_checkout}␟#{@drudwyn_recovery_checkout}␟#{@drudwyn_task_reference}",
    ])?;
    let mut observed = Vec::new();
    for row in panes.lines() {
        let fields: Vec<_> = row.split('␟').collect();
        if fields.len() != 9 {
            return Err(Error::Invalid(
                "Ambiguous pane metadata; recovery unavailable".into(),
            ));
        }
        // Read non-content cwd metadata once per pane. /proc preserves the
        // bytes that tmux escapes; other platforms use its metadata fallback.
        let cwd = fs::read_link(format!("/proc/{}/cwd", fields[1]))
            .unwrap_or_else(|_| PathBuf::from(fields[3]));
        let root = crate::workspace::git(&cwd, &["rev-parse", "--show-toplevel"]).ok();
        observed.push((fields, root));
    }
    let mut result = Vec::new();
    for record in data.split("\0\0").filter(|r| !r.is_empty()) {
        let fields: Vec<_> = record.split('\0').collect();
        let Some(path) = fields.iter().find_map(|f| f.strip_prefix("worktree ")) else {
            continue;
        };
        let path = PathBuf::from(path);
        let branch = fields
            .iter()
            .find_map(|f| f.strip_prefix("branch refs/heads/"))
            .unwrap_or("(detached)")
            .to_owned();
        let unavailable = if fields
            .iter()
            .any(|f| *f == "locked" || f.starts_with("locked "))
        {
            Some("Locked worktree")
        } else if fields
            .iter()
            .any(|f| *f == "prunable" || f.starts_with("prunable "))
        {
            Some("Prunable worktree; directory unavailable or registration stale")
        } else if !path.is_dir() {
            Some("Deleted directory")
        } else {
            None
        }
        .map(str::to_owned);
        let mut live = BTreeSet::new();
        let mut stopped = BTreeSet::new();
        let mut references = BTreeSet::new();
        for (f, root) in &observed {
            // A managed worker reserves its launch checkout even after cd.
            let exact = if f[6].is_empty() { f[7] } else { f[6] };
            let matches = decode(exact).as_ref().is_some_and(|p| *p == path)
                || Path::new(f[4]) == path
                || root.as_ref().is_some_and(|root| Path::new(root) == path);
            if matches {
                if f[2] == "0" {
                    live.insert(f[0].to_owned());
                } else {
                    stopped.insert(f[0].to_owned());
                }
                if !f[5].is_empty() {
                    references.insert(
                        decode(f[8])
                            .and_then(|p| p.to_str().map(str::to_owned))
                            .unwrap_or_else(|| f[5].to_owned()),
                    );
                }
            }
        }
        result.push(Checkout {
            path,
            branch,
            unavailable,
            live: live.into_iter().collect(),
            stopped: stopped.into_iter().collect(),
            task_file: (references.len() == 1).then(|| references.into_iter().next().unwrap()),
        });
    }
    Ok(result)
}

/// Recover presentation and project continuity only from surviving tmux
/// windows. Conflicting or absent metadata is deliberately not reconstructed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Context {
    pub project: String,
    pub name: String,
    pub batch: Option<String>,
}
pub(crate) fn context(checkout: &Checkout) -> Option<Context> {
    let mut result = None;
    for window in &checkout.stopped {
        let row = tmux(&[
            "display-message",
            "-p",
            "-t",
            window,
            "#{window_name}␟#{@drudwyn_project}␟#{@drudwyn_batch}",
        ])
        .ok()?;
        let f: Vec<_> = row.split('␟').collect();
        if f.len() != 3 || f[1].is_empty() {
            return None;
        }
        let repository = crate::coordinator::repository(&checkout.path).ok()?;
        let recorded = tmux(&["show-option", "-qv", "-t", f[1], "@drudwyn_project_repo"]).ok()?;
        if recorded != repository {
            return None;
        }
        let batch = if f[2].is_empty() {
            None
        } else {
            let b = crate::batch::load(f[2]).ok()?;
            if b.project != f[1] || b.repository != repository {
                return None;
            }
            Some(b.id)
        };
        let candidate = Context {
            project: f[1].into(),
            name: f[0].into(),
            batch,
        };
        if result.as_ref().is_some_and(|old| old != &candidate) {
            return None;
        }
        result = Some(candidate);
    }
    result
}

pub enum Task {
    None,
    Text(String),
    File(String),
    RetainedReference,
}

pub struct Request {
    pub repo: PathBuf,
    pub path: PathBuf,
    pub agent: Option<crate::domain::AgentKind>,
    pub task: Task,
    pub batch: Option<String>,
    pub coordinator: bool,
}

/// Recover only a selected existing checkout. Every failure retains Git files,
/// commits and any uncertain new terminal for deliberate inspection.
pub fn recover(request: Request) -> Result<crate::workspace::Started, Error> {
    if request.agent.is_none() && !matches!(request.task, Task::None) {
        return Err(Error::Invalid(
            "Open shell takes no task; select Restart with an agent".into(),
        ));
    }
    let client = crate::navigation::client()?;
    let repo = &request.repo;
    let path = &request.path;
    let select = || -> Result<Checkout, Error> {
        list(repo)?
            .into_iter()
            .find(|c| {
                c.path == *path
                    || c.path
                        .canonicalize()
                        .ok()
                        .zip(path.canonicalize().ok())
                        .is_some_and(|(a, b)| a == b)
            })
            .ok_or_else(|| {
                Error::Invalid(
                    "Path is not a registered worktree of the selected repository".into(),
                )
            })
    };
    let checkout = select()?;
    let retained = context(&checkout);
    let chosen_batch = request
        .batch
        .as_deref()
        .map(crate::batch::load)
        .transpose()?;
    let project = if let Some(b) = &chosen_batch {
        b.project.clone()
    } else if let Some(retained) = &retained {
        retained.project.clone()
    } else {
        crate::coordinator::project(&crate::navigation::current_session()?)?
    };
    if let Some(reason) = checkout.unavailable {
        return Err(Error::Invalid(reason));
    }
    let path = checkout.path.clone();
    let identity = crate::coordinator::repository(repo)?;
    let associated = tmux(&[
        "show-option",
        "-qv",
        "-t",
        &project,
        "@drudwyn_project_repo",
    ])?;
    if !associated.is_empty() && associated != identity {
        return Err(Error::Invalid(
            "Selected repository belongs to another project; switch to its project before recovery"
                .into(),
        ));
    }
    if crate::coordinator::repository(&path)? != identity {
        return Err(Error::Invalid(
            "Checkout repository changed; recovery cancelled".into(),
        ));
    }
    let directory = fs::File::open(&path)?;
    directory.try_lock().map_err(|_| {
        Error::Invalid("Recovery or delivery already in progress; retry after inspection".into())
    })?;
    let held = directory.metadata()?;
    let current = fs::metadata(&path)?;
    if held.dev() != current.dev() || held.ino() != current.ino() {
        return Err(Error::Invalid(
            "Checkout changed; recovery cancelled".into(),
        ));
    }
    let checkout = select()?;
    if let Some(reason) = checkout.unavailable {
        return Err(Error::Invalid(reason));
    }
    if !checkout.live.is_empty() {
        return Err(Error::Invalid(format!(
            "Live workspace already uses this checkout: {}. Open it instead",
            checkout.live.join(", ")
        )));
    }
    let coordinator_before = request
        .coordinator
        .then(|| crate::coordinator::missing(&project))
        .transpose()?;
    let batch = chosen_batch;
    if batch
        .as_ref()
        .is_some_and(|b| b.repository != identity || b.project != project)
    {
        return Err(Error::Invalid(
            "Selected batch belongs to another repository or project".into(),
        ));
    }
    let task = match request.task {
        Task::RetainedReference => Task::File(checkout.task_file.ok_or_else(|| {
            Error::Invalid(
                "Task reference Unknown; explicitly select a task/reference after metadata loss"
                    .into(),
            )
        })?),
        task => task,
    };
    match &task {
        Task::None if request.agent.is_some() => {
            return Err(Error::Invalid(
                "Restart needs an explicit task/reference; historical prompts are Unknown".into(),
            ));
        }
        Task::Text(text) if text.trim().is_empty() => {
            return Err(Error::Invalid("Task is empty; no restart".into()));
        }
        Task::File(reference) => crate::workspace::validate_task_file(&path, reference)?,
        _ => {}
    }
    let command = if let Some(agent) = request.agent {
        agent_command(agent)?
    } else {
        vec![tmux(&["show-option", "-gv", "default-shell"])?, "-l".into()]
    };
    let started = crate::workspace::launch_existing(
        crate::workspace::Launch {
            path: path.clone(),
            repo: repo.to_owned(),
            branch: checkout.branch.clone(),
            name: if request.coordinator {
                "Coordinator".into()
            } else {
                retained
                    .as_ref()
                    .map(|c| c.name.clone())
                    .unwrap_or(checkout.branch)
            },
            command,
            project: Some(project.clone()),
            batch,
            task_file: match &task {
                Task::File(file) => Some(file.clone()),
                _ => None,
            },
            allocated_commit: None,
            track_agent: request.agent.is_some(),
        },
        Some(&directory),
    )?;
    let current = fs::metadata(&path)?;
    if held.dev() != current.dev()
        || held.ino() != current.ino()
        || crate::coordinator::repository(&path)? != identity
    {
        return Err(Error::Invalid(format!(
            "Checkout changed during recovery; retained {}. Inspect before retry; no task sent",
            started.window_id
        )));
    }
    let checkout = select()?;
    if checkout.unavailable.is_some() {
        return Err(Error::Invalid(format!(
            "Worktree became unavailable; retained {}. No task sent",
            started.window_id
        )));
    }
    if checkout.live.iter().any(|w| *w != started.window_id) {
        return Err(Error::Invalid(format!(
            "Another window appeared during recovery; retained {}. Inspect before retry; no task sent",
            started.window_id
        )));
    }
    if let Some(expected) = coordinator_before {
        crate::coordinator::restore(&started.window_id, &project, &expected)?;
    }
    drop(directory); // Delivery reacquires this inode and revalidates its binding.
    match task {
        Task::Text(task) => crate::workspace::send_started(&started, &task, false, request.agent)?,
        Task::File(task) => crate::workspace::send_started(&started, &task, true, request.agent)?,
        _ => {}
    }
    crate::navigation::open_for(&client, Some(&started.window_id), None)?;
    let message = if request.agent.is_some() {
        "Worker restarted; task sent. Fresh conversation; acceptance unconfirmed. Prefix + C returns to coordinator."
    } else {
        "Workspace shell opened. Prefix + C returns to coordinator."
    };
    let _ = tmux(&["display-message", "-c", &client, "-d", "6000", message]);
    Ok(started)
}

pub(crate) fn agent_command(agent: crate::domain::AgentKind) -> Result<Vec<String>, Error> {
    // Resolve once in the invoking command's environment, then exec this
    // exact path. Never silently fall back to an installed different agent.
    let output = Command::new("sh")
        .args([
            "-c",
            "command -v \"$1\"",
            "drudwyn-recovery",
            agent.command(),
        ])
        .output()?;
    let executable = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if !output.status.success() || executable.is_empty() || !Path::new(&executable).is_file() {
        return Err(Error::Invalid(format!(
            "Agent {} unavailable; no restart",
            agent.command()
        )));
    }
    let executable = PathBuf::from(executable);
    let executable = if executable.is_absolute() {
        executable
    } else {
        std::env::current_dir()?.join(executable)
    };
    Ok(vec![executable.to_string_lossy().into_owned()])
}

// Lossless operational path metadata, never a guess at tmux display escaping.
pub(crate) fn encode(path: &Path) -> String {
    path.as_os_str()
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
pub(crate) fn decode(identity: &str) -> Option<PathBuf> {
    if identity.is_empty()
        || identity.len() % 2 != 0
        || !identity.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return None;
    }
    let bytes = (0..identity.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&identity[i..i + 2], 16).ok())
        .collect::<Option<Vec<_>>>()?;
    Some(PathBuf::from(OsString::from_vec(bytes)))
}
