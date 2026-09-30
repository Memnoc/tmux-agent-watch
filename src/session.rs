//! Ad hoc shell sessions. Inputs stay argument data; Git and agents are untouched.
use std::{io, path::Path};

use crate::navigation::{self, tmux};

// tmux parses a trailing semicolon as a command separator even in argv.
fn argument(value: &str) -> String {
    match value.strip_suffix(';') {
        Some(prefix) => format!("{prefix}\\;"),
        None => value.to_owned(),
    }
}

/// The requesting terminal's selected workspace supplies the starting directory.
pub fn directory() -> io::Result<String> {
    let client = navigation::client()?;
    directory_for(&client)
}

fn directory_for(client: &str) -> io::Result<String> {
    let rows = tmux(&["list-clients", "-F", "#{client_name}␟#{pane_current_path}␟"])?;
    rows.lines()
        .filter_map(|row| row.split_once('␟'))
        .find(|(name, _)| *name == client)
        .map(|(_, directory)| directory.strip_suffix('␟').unwrap_or(directory).to_owned())
        .ok_or_else(|| io::Error::other("Requesting client detached"))
}

/// Create a shell and open it only in the requesting terminal.
pub fn create(name: &str, directory: Option<&Path>) -> io::Result<String> {
    let client = navigation::client()?;
    if name.trim().is_empty() || name.chars().any(|c| c.is_control() || ":.␟".contains(c)) {
        return Err(io::Error::other(
            "Invalid session name: use a nonempty name without dots, colons, or control characters",
        ));
    }
    let default_directory;
    let directory = match directory {
        Some(directory) => directory,
        None => {
            default_directory = directory_for(&client)?;
            Path::new(&default_directory)
        }
    };
    let directory = directory
        .canonicalize()
        .map_err(|error| io::Error::other(format!("Invalid starting directory: {error}")))?;
    if !directory.is_dir() {
        return Err(io::Error::other("Starting directory must be a directory"));
    }
    let directory = directory
        .to_str()
        .ok_or_else(|| io::Error::other("Starting directory must be valid UTF-8"))?;
    // -c and -s are tmux formats; escape hashes as well as argv separators.
    let directory = argument(&directory.replace('#', "##"));
    // tmux stores backslashes doubled in session names. Match that native
    // spelling only for this queued target; creation still receives the input
    // name, and every later action uses the returned stable session ID.
    let exact_name = format!("={}:", name.replace('\\', "\\\\"));
    let name = argument(&name.replace('#', "##"));
    let shell = tmux(&["show-option", "-gv", "default-shell"])?;
    let session = tmux(&[
        "new-session",
        "-d",
        "-s",
        &name,
        "-c",
        &directory,
        "-P",
        "-F",
        "#{session_id}",
        &shell,
        "-l",
        ";",
        "set-option",
        "-t",
        &exact_name,
        "destroy-unattached",
        "off",
    ])
    .map_err(|error| {
        io::Error::other(format!(
            "Session creation failed: {error}. If creation partly succeeded, inspect the session navigator before retrying"
        ))
    })?;
    if !session.starts_with('$')
        || session.len() < 2
        || !session[1..].bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(io::Error::other(
            "Creation returned no stable session ID; the new session may remain. Inspect the session navigator before retrying",
        ));
    }
    if let Err(error) = navigation::open_for(&client, None, Some(&session)) {
        let cleanup = tmux(&["kill-session", "-t", &session]);
        return Err(io::Error::other(match cleanup {
            Ok(_) => format!("Switch failed: {error}; new session removed, retry creation"),
            Err(cleanup) => {
                format!("Switch failed: {error}; new session {session} remains: {cleanup}")
            }
        }));
    }
    // Restore the user's inherited cleanup policy only after attachment.
    tmux(&["set-option", "-u", "-t", &session, "destroy-unattached"]).map_err(|error| {
        io::Error::other(format!(
            "Created and opened session {session}, but restoring its cleanup policy failed: {error}"
        ))
    })?;
    Ok(session)
}
