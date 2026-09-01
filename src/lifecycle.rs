use std::{
    env,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use thiserror::Error;

use crate::domain::{AgentKind, Lifecycle};

const SEPARATOR: char = '\u{241f}';
const PANE_FORMAT: &str = "#{window_id}␟#{pane_id}␟#{pane_current_command}␟#{pane_dead}␟#{@agent_watch_state}␟#{@agent_watch_source}";

#[derive(Debug, Error)]
pub enum LifecycleError {
    #[error("could not invoke tmux: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("tmux command failed")]
    Tmux,
    #[error("TMUX_PANE is not available")]
    MissingPane,
    #[error("unsupported lifecycle event: {0}")]
    UnsupportedEvent(String),
}

pub fn scan() -> Result<(), LifecycleError> {
    let output = tmux_output(&["list-panes", "-a", "-F", PANE_FORMAT])?;
    for line in output.lines().filter(|line| !line.is_empty()) {
        let fields = line.split(SEPARATOR).collect::<Vec<_>>();
        if fields.len() != 6 {
            continue;
        }
        let window_id = fields[0];
        let agent = AgentKind::from_command(fields[2]);
        if agent.is_none() {
            if !fields[4].is_empty() && fields[5] != "hook" {
                clear(window_id)?;
            }
            continue;
        }

        // Remove content retained by v1 as soon as v2 observes a workspace.
        set_option(window_id, "@agent_watch_message", "")?;
        if fields[3] == "1" {
            set_state(window_id, Lifecycle::Failed, "process")?;
        } else if fields[4].is_empty() || fields[5] != "hook" {
            set_state(window_id, Lifecycle::Working, "process")?;
        }
    }
    Ok(())
}

pub fn hook(agent: AgentKind, event: &str) -> Result<(), LifecycleError> {
    let lifecycle = map_event(agent, event)
        .ok_or_else(|| LifecycleError::UnsupportedEvent(event.to_owned()))?;
    let pane = env::var("TMUX_PANE").map_err(|_| LifecycleError::MissingPane)?;
    let window_id = tmux_output(&["display-message", "-p", "-t", &pane, "#{window_id}"])?;
    if window_id.is_empty() {
        return Ok(());
    }
    set_state(&window_id, lifecycle, "hook")
}

pub fn map_event(agent: AgentKind, event: &str) -> Option<Lifecycle> {
    let normalised = event.to_ascii_lowercase().replace(['_', '-'], "");
    match (agent, normalised.as_str()) {
        (AgentKind::Codex, "userpromptsubmit") => Some(Lifecycle::Working),
        (AgentKind::Codex, "permissionrequest" | "interrupt") => Some(Lifecycle::Waiting),
        (AgentKind::Codex, "stop") => Some(Lifecycle::Review),
        (AgentKind::Claude, "userpromptsubmit") => Some(Lifecycle::Working),
        (AgentKind::Claude, "permissionrequest" | "permissionprompt") => Some(Lifecycle::Waiting),
        (AgentKind::Claude, "stop" | "idleprompt") => Some(Lifecycle::Review),
        (AgentKind::Claude, "stopfailure") => Some(Lifecycle::Failed),
        (AgentKind::OpenCode, "working") => Some(Lifecycle::Working),
        (AgentKind::OpenCode, "permission") => Some(Lifecycle::Waiting),
        (AgentKind::OpenCode, "idle") => Some(Lifecycle::Review),
        (AgentKind::OpenCode, "error") => Some(Lifecycle::Failed),
        _ => None,
    }
}

fn set_state(window_id: &str, lifecycle: Lifecycle, source: &str) -> Result<(), LifecycleError> {
    let state = match lifecycle {
        Lifecycle::Starting => "starting",
        Lifecycle::Working => "working",
        Lifecycle::Waiting => "needs_input",
        Lifecycle::Review => "done",
        Lifecycle::Failed => "failed",
        Lifecycle::Unknown => "",
    };
    let previous = tmux_output(&["show-option", "-wqv", "-t", window_id, "@agent_watch_state"])?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string();
    if previous != state {
        set_option(window_id, "@agent_watch_since", &now)?;
        if lifecycle.needs_attention() {
            set_option(window_id, "@agent_watch_attention_since", &now)?;
        } else {
            set_option(window_id, "@agent_watch_attention_since", "")?;
        }
    }
    let (symbol, color) = match lifecycle {
        Lifecycle::Working | Lifecycle::Starting => ("●", "#9ccfd8"),
        Lifecycle::Waiting => ("●", "#f6c177"),
        Lifecycle::Review => ("●", "#a6da95"),
        Lifecycle::Failed => ("●", "#ed8796"),
        Lifecycle::Unknown => ("", "default"),
    };
    set_option(window_id, "@agent_watch_state", state)?;
    set_option(window_id, "@agent_watch_source", source)?;
    set_option(window_id, "@agent_watch_message", "")?;
    set_option(
        window_id,
        "@agent_watch_marker",
        &format!("#[fg={color}]{symbol}#[default] "),
    )?;
    set_option(
        window_id,
        "@agent_watch_window_style",
        &format!("#[fg={color}]"),
    )?;
    Ok(())
}

fn clear(window_id: &str) -> Result<(), LifecycleError> {
    for option in [
        "@agent_watch_state",
        "@agent_watch_source",
        "@agent_watch_message",
        "@agent_watch_since",
        "@agent_watch_attention_since",
        "@agent_watch_marker",
        "@agent_watch_window_style",
    ] {
        set_option(window_id, option, "")?;
    }
    Ok(())
}

fn set_option(window_id: &str, name: &str, value: &str) -> Result<(), LifecycleError> {
    tmux_status(&["set-option", "-wq", "-t", window_id, name, value])
}

fn tmux_output(args: &[&str]) -> Result<String, LifecycleError> {
    let output = Command::new("tmux").args(args).output()?;
    if !output.status.success() {
        return Err(LifecycleError::Tmux);
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn tmux_status(args: &[&str]) -> Result<(), LifecycleError> {
    Command::new("tmux")
        .args(args)
        .status()?
        .success()
        .then_some(())
        .ok_or(LifecycleError::Tmux)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapters_map_events_without_payloads() {
        assert_eq!(
            map_event(AgentKind::Codex, "permissionRequest"),
            Some(Lifecycle::Waiting)
        );
        assert_eq!(
            map_event(AgentKind::Claude, "StopFailure"),
            Some(Lifecycle::Failed)
        );
        assert_eq!(
            map_event(AgentKind::OpenCode, "idle"),
            Some(Lifecycle::Review)
        );
        assert_eq!(map_event(AgentKind::Codex, "prompt text"), None);
    }

    #[test]
    fn scan_format_requests_no_content() {
        for forbidden in [
            "capture-pane",
            "message",
            "history",
            "title",
            "command_arguments",
        ] {
            assert!(!PANE_FORMAT.contains(forbidden), "requested {forbidden}");
        }
    }
}
