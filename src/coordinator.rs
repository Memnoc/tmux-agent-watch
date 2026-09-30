//! Explicit project associations live only in tmux. Session IDs distinguish
//! projects even when their repository or window display names are identical.
use crate::navigation::{self, tmux};
use std::{io, path::Path, process::Command};

pub fn project(session: &str) -> io::Result<String> {
    let rows = tmux(&["list-sessions", "-F", "#{session_id}␟#{@drudwyn_view_of}"])?;
    rows.lines()
        .filter_map(|row| row.split_once('␟'))
        .find(|(id, _)| *id == session)
        .map(|(id, anchor)| if anchor.is_empty() { id } else { anchor }.to_owned())
        .ok_or_else(|| io::Error::other("Project session has vanished"))
}

pub fn repository(path: &Path) -> io::Result<String> {
    let result = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
        .output()?;
    if !result.status.success() {
        return Err(io::Error::other("Coordinator requires a Git checkout"));
    }
    Ok(String::from_utf8_lossy(&result.stdout).trim().into())
}

pub fn set(window: &str, session: Option<&str>) -> io::Result<()> {
    let session = match session {
        Some(id) => id.to_owned(),
        None => navigation::context("#{session_id}")?,
    };
    let project = project(&session)?;
    let rows = tmux(&[
        "list-windows",
        "-a",
        "-F",
        "#{session_id}␟#{window_id}␟#{pane_current_path}␟#{@drudwyn_launch_checkout}␟#{@drudwyn_recovery_checkout}",
    ])?;
    let row = rows
        .lines()
        .map(|r| r.split('␟').collect::<Vec<_>>())
        .find(|r| r[0] == project && r[1] == window)
        .ok_or_else(|| io::Error::other("Coordinator window is not a member of this project"))?;
    // Use a lossless known checkout identity, never unescape tmux display cwd.
    let exact = if row[3].is_empty() { row[4] } else { row[3] };
    let path = crate::recovery::decode(exact).unwrap_or_else(|| row[2].into());
    let repo = repository(&path)?;
    let previous = tmux(&[
        "show-option",
        "-qv",
        "-t",
        &project,
        "@drudwyn_project_repo",
    ])?;
    if !previous.is_empty() && previous != repo {
        return Err(io::Error::other(
            "Project is associated with a different repository; use a separate session",
        ));
    }
    // Preserve the user's current name. Disable process-driven renames without
    // periodically rewriting it, so future explicit renames remain authoritative.
    protect_name(window)?;
    tmux(&["set-option", "-t", &project, "@drudwyn_project_repo", &repo])?;
    tmux(&["set-option", "-t", &project, "@drudwyn_coordinator", window])?;
    tmux(&[
        "set-option",
        "-w",
        "-t",
        window,
        "@drudwyn_project",
        &project,
    ])?;
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
