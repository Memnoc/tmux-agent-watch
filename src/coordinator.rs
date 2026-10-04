//! Explicit project associations live only in tmux. Session IDs distinguish
//! projects even when their repository or window display names are identical.
use crate::navigation::{self, tmux};
use std::{io, path::Path};

pub fn project(session: &str) -> io::Result<String> {
    let rows = tmux(&["list-sessions", "-F", "#{session_id}␟#{@drudwyn_view_of}"])?;
    rows.lines()
        .filter_map(|row| row.split_once('␟'))
        .find(|(id, _)| *id == session)
        .map(|(id, anchor)| if anchor.is_empty() { id } else { anchor }.to_owned())
        .ok_or_else(|| io::Error::other("Project session has vanished"))
}

pub fn repository(path: &Path) -> io::Result<String> {
    let result = crate::workspace::checkout_git(
        path,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    )
    .output()?;
    if !result.status.success() {
        return Err(io::Error::other("Coordinator requires a Git checkout"));
    }
    Ok(String::from_utf8_lossy(&result.stdout).trim().into())
}

fn ownership(project: &str) -> io::Result<(String, bool)> {
    let current = tmux(&["show-option", "-qv", "-t", project, "@drudwyn_coordinator"])?;
    let windows = tmux(&["list-windows", "-t", project, "-F", "#{window_id}"])?;
    let live = !current.is_empty() && windows.lines().any(|w| w == current);
    Ok((current, live))
}

/// Observe absent ownership under the same guard used for final assignment.
/// Release it during launch: a deliberate intervening selection must win.
pub(crate) fn missing(project: &str) -> io::Result<String> {
    let _guard = crate::lifecycle::LifecycleGuard::acquire().map_err(io::Error::other)?;
    let (current, live) = ownership(project)?;
    if live {
        return Err(io::Error::other(
            "Coordinator still exists; open it instead",
        ));
    }
    Ok(current)
}

pub fn set(window: &str, session: Option<&str>) -> io::Result<()> {
    let session = match session {
        Some(id) => id.to_owned(),
        None => navigation::context("#{session_id}")?,
    };
    let project = project(&session)?;
    let guard = crate::lifecycle::LifecycleGuard::acquire().map_err(io::Error::other)?;
    assign(&guard, window, &project)
}

/// Compare and assign atomically with other Drudwyn coordinator writers.
/// Recovery holds its checkout guard first, then this socket guard. Coordinator
/// setters never acquire checkout guards; no reverse lock ordering is allowed.
pub(crate) fn restore(window: &str, project: &str, expected: &str) -> io::Result<()> {
    let guard = crate::lifecycle::LifecycleGuard::acquire().map_err(io::Error::other)?;
    let (current, live) = ownership(project)?;
    if current != expected || live {
        return Err(io::Error::other(format!(
            "Coordinator changed during recovery; retained window {window} and checkout. No task sent; open the chosen coordinator instead"
        )));
    }
    assign(&guard, window, project)
}

/// Explicit conflict recovery may select a new coordinator while retaining the
/// previous window. A concurrent deliberate selection must still win.
pub(crate) fn replace_after_conflict_recovery(
    window: &str,
    project: &str,
    expected: &str,
) -> io::Result<()> {
    let guard = crate::lifecycle::LifecycleGuard::acquire().map_err(io::Error::other)?;
    let (current, _) = ownership(project)?;
    if current != expected {
        return Err(io::Error::other(
            "Coordinator changed during recovery; new window retained, task not sent; inspect before retry",
        ));
    }
    assign(&guard, window, project)
}

fn assign(guard: &crate::lifecycle::LifecycleGuard, window: &str, project: &str) -> io::Result<()> {
    let rows = tmux(&[
        "list-windows",
        "-t",
        project,
        "-F",
        "#{window_id}␟#{pane_id}",
    ])?;
    let pane = rows
        .lines()
        .filter_map(|r| r.split_once('␟'))
        .find(|(id, _)| *id == window)
        .map(|(_, pane)| pane)
        .ok_or_else(|| io::Error::other("Coordinator window is not a member of this project"))?;
    let path = crate::recovery::selected_checkout(window, pane).map_err(io::Error::other)?;
    let repo = repository(&path)?;
    let previous = tmux(&["show-option", "-qv", "-t", project, "@drudwyn_project_repo"])?;
    if !previous.is_empty() && previous != repo {
        return Err(io::Error::other(
            "Project is associated with a different repository; use a separate session",
        ));
    }
    for option in ["automatic-rename", "allow-rename"] {
        guard
            .mutation(&["set-option", "-w", "-t", window, option, "off"])
            .map_err(io::Error::other)?;
    }
    for args in [
        vec!["set-option", "-t", project, "@drudwyn_project_repo", &repo],
        vec!["set-option", "-t", project, "@drudwyn_coordinator", window],
        vec![
            "set-option",
            "-w",
            "-t",
            window,
            "@drudwyn_project",
            project,
        ],
    ] {
        guard.mutation(&args).map_err(io::Error::other)?;
    }
    Ok(())
}

pub fn protect_name(window: &str) -> io::Result<()> {
    for option in ["automatic-rename", "allow-rename"] {
        tmux(&["set-option", "-w", "-t", window, option, "off"])?;
    }
    Ok(())
}

pub fn open_project(session: &str) -> io::Result<()> {
    let project = project(session)?;
    let coordinator = tmux(&["show-option", "-qv", "-t", &project, "@drudwyn_coordinator"])?;
    let windows = tmux(&["list-windows", "-t", &project, "-F", "#{window_id}"])?;
    if coordinator.is_empty() {
        return Err(io::Error::other(
            "Coordinator unknown; select a shell or agent and use coordinator set --window ID",
        ));
    }
    if !windows.lines().any(|id| id == coordinator) {
        return Err(io::Error::other(
            "Coordinator unavailable; recover by opening a shell in the project checkout, then use coordinator set --window ID",
        ));
    }
    navigation::open(Some(&coordinator), Some(&project))
}

pub fn open_window(window: &str) -> io::Result<()> {
    let project = tmux(&["show-option", "-wqv", "-t", window, "@drudwyn_project"])?;
    if project.is_empty() {
        return Err(io::Error::other(
            "Coordinator association unknown for this workspace",
        ));
    }
    open_project(&project)
}

/// Inherit only an explicit project association with the same Git identity.
/// Detached legacy callers may launch unassociated work; an explicitly supplied
/// client must resolve correctly before allocating a checkout.
pub fn launch_project(repo: &Path) -> io::Result<Option<String>> {
    if std::env::var_os("DRUDWYN_CLIENT").is_some() {
        navigation::client()?;
    }
    let session = navigation::current_session()?;
    if session.is_empty() {
        return Ok(None);
    }
    let project = project(&session)?;
    let recorded = tmux(&[
        "show-option",
        "-qv",
        "-t",
        &project,
        "@drudwyn_project_repo",
    ])?;
    if recorded.is_empty() || recorded != repository(repo)? {
        return Ok(None);
    }
    Ok(Some(project))
}

/// One shortcut switches between a project's coordinator and the last worker
/// visited by this client. The bookmark is live tmux metadata, keyed by client.
pub fn toggle() -> io::Result<()> {
    let client = navigation::client()?;
    let rows = tmux(&[
        "list-clients",
        "-F",
        "#{client_name}␟#{session_id}␟#{window_id}",
    ])?;
    let row = rows
        .lines()
        .filter_map(|l| {
            let f: Vec<_> = l.split('␟').collect();
            (f.len() == 3 && f[0] == client).then_some(f)
        })
        .next()
        .ok_or_else(|| io::Error::other("Requesting client disappeared"))?;
    let project = project(row[1])?;
    let coordinator = tmux(&["show-option", "-qv", "-t", &project, "@drudwyn_coordinator"])?;
    let key = format!(
        "@drudwyn_return_{}",
        client
            .bytes()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    if row[2] == coordinator {
        let worker = tmux(&["show-option", "-qv", "-t", &project, &key])?;
        let windows = tmux(&["list-windows", "-t", &project, "-F", "#{window_id}"])?;
        if worker.is_empty() || !windows.lines().any(|id| id == worker) {
            return Err(io::Error::other(
                "No previous worker available. Open a worker first.",
            ));
        }
        navigation::open_for(&client, Some(&worker), Some(&project))
    } else {
        open_project(&project)?;
        tmux(&["set-option", "-q", "-t", &project, &key, row[2]])?;
        Ok(())
    }
}
