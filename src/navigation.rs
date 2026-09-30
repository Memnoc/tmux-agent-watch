//! Client-local navigation over shared tmux windows. Only application-created
//! views carry `@drudwyn_view_of`; unrelated grouped sessions remain user-owned.
use std::{
    io,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

pub(crate) fn tmux(args: &[&str]) -> io::Result<String> {
    let output = Command::new("tmux").args(args).output()?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim_end().into())
    } else {
        Err(io::Error::other(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ))
    }
}

pub fn client() -> io::Result<String> {
    let clients = tmux(&["list-clients", "-F", "#{client_name}␟#{session_id}"])?;
    let explicit = std::env::var("DRUDWYN_CLIENT").ok();
    let pane_sessions = if explicit.is_none() {
        std::env::var("TMUX_PANE")
            .ok()
            .map(|pane| {
                tmux(&["list-panes", "-a", "-F", "#{pane_id}␟#{session_id}"]).map(|output| {
                    output
                        .lines()
                        .filter_map(|line| line.split_once('␟'))
                        .filter(|(id, _)| *id == pane)
                        .map(|(_, session)| session.to_owned())
                        .collect::<Vec<_>>()
                })
            })
            .transpose()?
    } else {
        None
    };
    let candidates: Vec<_> = clients
        .lines()
        .filter_map(|line| line.split_once('␟'))
        .filter(|(name, session)| match &explicit {
            Some(requested) => name == requested,
            None => pane_sessions
                .as_ref()
                .is_none_or(|sessions| sessions.iter().any(|current| current == session)),
        })
        .collect();
    match candidates.as_slice() {
        [(name, _)] => Ok((*name).into()),
        [] => Err(io::Error::other(format!(
            "Requesting client is missing or detached: {}",
            explicit.as_deref().unwrap_or("inferred")
        ))),
        _ => Err(io::Error::other(
            "Requesting client is ambiguous; supply --client or DRUDWYN_CLIENT",
        )),
    }
}

fn stable(value: &str, prefix: char) -> bool {
    value.starts_with(prefix) && value.len() > 1 && value[1..].bytes().all(|c| c.is_ascii_digit())
}

/// Resolve both memberships and client before mutating anything. A private
/// grouped view shares the actual windows/panes, never their processes or files.
pub fn open(window: Option<&str>, session: Option<&str>) -> io::Result<()> {
    let client = client()?;
    if window.is_some_and(|id| !stable(id, '@')) || session.is_some_and(|id| !stable(id, '$')) {
        return Err(io::Error::other(
            "Navigation requires a stable window/session ID",
        ));
    }
    let sessions = tmux(&[
        "list-sessions",
        "-F",
        "#{session_id}␟#{session_name}␟#{@drudwyn_view_of}",
    ])?;
    let rows: Vec<Vec<_>> = sessions
        .lines()
        .map(|line| line.split('␟').collect())
        .collect();
    let clients = tmux(&["list-clients", "-F", "#{client_name}␟#{session_id}"])?;
    let current = clients
        .lines()
        .filter_map(|line| line.split_once('␟'))
        .find(|(name, _)| *name == client)
        .ok_or_else(|| io::Error::other("Requesting client detached"))?
        .1
        .to_owned();
    let memberships = tmux(&["list-windows", "-a", "-F", "#{session_id}␟#{window_id}"])?;
    let members: Vec<_> = memberships
        .lines()
        .filter_map(|line| line.split_once('␟'))
        .filter(|(_, id)| window.is_none_or(|window| window == *id))
        .map(|(id, _)| id)
        .collect();
    let target = if let Some(session) = session {
        if !rows.iter().any(|row| row[0] == session) || !members.contains(&session) {
            return Err(io::Error::other(
                "Target session or window membership has vanished",
            ));
        }
        session.to_owned()
    } else if members.contains(&current.as_str()) {
        current.clone()
    } else {
        members
            .iter()
            .find(|id| rows.iter().any(|row| row[0] == **id && row[2].is_empty()))
            .or(members.first())
            .ok_or_else(|| io::Error::other("Target window has vanished"))?
            .to_string()
    };
    let row = rows
        .iter()
        .find(|row| row[0] == target)
        .ok_or_else(|| io::Error::other("Target session has vanished"))?;
    let anchor = if row[2].is_empty() { row[0] } else { row[2] };
    let current_is_view = rows.iter().any(|row| row[0] == current && row[2] == anchor);
    let clients = tmux(&["list-clients", "-F", "#{client_name}␟#{session_id}"])?;
    let busy = |id: &str| {
        clients
            .lines()
            .filter_map(|line| line.split_once('␟'))
            .any(|(name, session)| session == id && name != client)
    };
    let mut destination = if current_is_view && !busy(&current) {
        current.clone()
    } else {
        target.clone()
    };
    let mut created = false;
    if busy(&destination) {
        // Protect the new view in the same command queue as its creation:
        // inherited destroy-unattached may remove it before a second tmux call.
        // A unique exact name lets us address it before receiving its stable ID.
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let name = format!("drudwyn-view-{}-{nonce}", std::process::id());
        let exact_name = format!("={name}:");
        destination = tmux(&[
            "new-session",
            "-d",
            "-s",
            &name,
            "-P",
            "-F",
            "#{session_id}",
            "-t",
            &target,
            ";",
            "set-option",
            "-t",
            &exact_name,
            "destroy-unattached",
            "off",
        ])?;
        if let Err(error) = tmux(&["set-option", "-t", &destination, "@drudwyn_view_of", anchor]) {
            let _ = tmux(&["kill-session", "-t", &destination]);
            return Err(error);
        }
        created = true;
    }
    let target = window
        .map(|window| format!("{destination}:{window}"))
        .unwrap_or(destination.clone());
    let result = tmux(&["switch-client", "-c", &client, "-t", &target]);
    if result.is_err() && created {
        let _ = tmux(&["kill-session", "-t", &destination]);
    }
    result?;
    if created {
        // Set after attachment: a detached view would otherwise vanish at birth.
        tmux(&["set-option", "-t", &destination, "destroy-unattached", "on"])?;
    }
    Ok(())
}

/// Application view names map back to their shared project's live session name.
/// User-created grouped sessions have no ownership marker and remain visible.
pub(crate) fn view_names() -> io::Result<std::collections::HashMap<String, String>> {
    let output = tmux(&[
        "list-sessions",
        "-F",
        "#{session_id}␟#{session_name}␟#{@drudwyn_view_of}",
    ])?;
    let rows: Vec<Vec<_>> = output
        .lines()
        .map(|line| line.split('␟').collect())
        .collect();
    Ok(rows
        .iter()
        .filter_map(|row| {
            rows.iter()
                .find(|anchor| anchor[0] == row[2])
                .map(|anchor| (row[1].to_owned(), anchor[1].to_owned()))
        })
        .collect())
}

/// Read selection from the requesting client rather than tmux's default target.
/// Detached UI fixtures may still browse using their explicitly identified pane.
pub(crate) fn context(format: &str) -> io::Result<String> {
    if let Ok(client) = client() {
        let rows = tmux(&["list-clients", "-F", &format!("#{{client_name}}␟{format}")])?;
        return rows
            .lines()
            .filter_map(|line| line.split_once('␟'))
            .find(|(name, _)| *name == client)
            .map(|(_, value)| value.to_owned())
            .ok_or_else(|| io::Error::other("Requesting client detached"));
    }
    if let Ok(pane) = std::env::var("TMUX_PANE") {
        return tmux(&["display-message", "-p", "-t", &pane, format]);
    }
    Ok(String::new())
}
