use std::{
    collections::BTreeMap,
    env,
    fs::{self, File},
    os::unix::fs::MetadataExt,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use thiserror::Error;

use crate::domain::{AgentKind, Lifecycle};

const SEPARATOR: char = '\u{241f}';
const PANE_FORMAT: &str = "#{window_id}␟#{pane_id}␟#{pane_pid}␟#{pane_current_command}␟#{pane_dead}␟#{pane_dead_status}␟#{pane_dead_time}␟#{@drudwyn_p_identity}␟#{@drudwyn_p_agent}␟#{@drudwyn_p_state}␟#{@drudwyn_p_source}␟#{@drudwyn_p_since}␟#{@drudwyn_p_attention_since}␟#{@drudwyn_launch_pane}␟#{@drudwyn_launch_pid}␟#{pane_dead_signal}␟#{@drudwyn_launch_stage}";

#[derive(Debug, Error)]
pub enum LifecycleError {
    #[error("could not invoke tmux: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("tmux command failed")]
    Tmux,
    #[error("TMUX_PANE is not available")]
    MissingPane,
    #[error("Lifecycle event has no unique matching live agent in its originating pane")]
    Unattributed,
    #[error("Lifecycle event's observed agent changed in its originating pane")]
    ChangedAgent,
    #[error("unsupported lifecycle event: {0}")]
    UnsupportedEvent(String),
    #[error("could not serialize lifecycle updates: {0}")]
    Synchronization(String),
}

/// Lock the existing socket directory, without creating a lock file or storing
/// state. Servers sharing a directory serialize conservatively. The descriptor
/// is close-on-exec except for controlled mutation children, which hold a clone
/// as their unused stdin until they finish. Parent death cannot release the lock
/// while an already-spawned child can still write stale evidence.
/// All lifecycle snapshots and writes, including window projection, must be
/// inside this guard. A reread alone would leave another read/write race.
struct LifecycleGuard {
    directory: File,
}

impl LifecycleGuard {
    fn acquire() -> Result<Self, LifecycleError> {
        let socket = tmux_output(&["display-message", "-p", "#{socket_path}"])?;
        let path = Path::new(&socket);
        let parent = path
            .parent()
            .filter(|_| path.is_absolute())
            .ok_or_else(|| {
                LifecycleError::Synchronization("tmux socket directory is unavailable".into())
            })?;
        let unavailable =
            |error: std::io::Error| LifecycleError::Synchronization(error.to_string());
        let directory = File::open(parent).map_err(unavailable)?;
        let held = directory.metadata().map_err(unavailable)?;
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            match directory.try_lock() {
                Ok(()) => break,
                Err(fs::TryLockError::WouldBlock) if Instant::now() < deadline => {
                    thread::sleep(Duration::from_millis(10));
                }
                Err(fs::TryLockError::WouldBlock) => {
                    return Err(LifecycleError::Synchronization(
                        "another update is still in progress; retry this command".into(),
                    ));
                }
                Err(fs::TryLockError::Error(error)) => return Err(unavailable(error)),
            }
        }
        let current = fs::metadata(parent).map_err(unavailable)?;
        if !held.is_dir() || held.dev() != current.dev() || held.ino() != current.ino() {
            return Err(LifecycleError::Synchronization(
                "tmux socket directory changed; retry this command".into(),
            ));
        }
        Ok(Self { directory })
    }
}

// Only executable names, process ancestry and birth times are observed. In
// particular, never request argv, environment, or terminal output from ps.
struct Process {
    pid: String,
    parent: String,
    birth: String,
    agent: Option<AgentKind>,
}
fn processes() -> Result<Vec<Process>, LifecycleError> {
    let output = Command::new("ps")
        .args(["-eo", "pid=,ppid=,lstart=,comm="])
        .output()?;
    if !output.status.success() {
        return Err(LifecycleError::Tmux);
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| {
            let f: Vec<_> = line.split_whitespace().collect();
            (f.len() >= 8).then(|| Process {
                pid: f[0].into(),
                parent: f[1].into(),
                birth: f[2..7].join("-"),
                agent: AgentKind::from_command(f[7]),
            })
        })
        .collect())
}
fn belongs_to(process: &Process, root: &str, all: &[Process]) -> bool {
    let mut current = process;
    for _ in 0..128 {
        if current.pid == root {
            return true;
        }
        let Some(parent) = all.iter().find(|p| p.pid == current.parent) else {
            return false;
        };
        if parent.pid == current.pid {
            return false;
        }
        current = parent;
    }
    false
}

pub fn scan() -> Result<(), LifecycleError> {
    let guard = LifecycleGuard::acquire()?;
    reconcile(&guard)
}

fn reconcile(guard: &LifecycleGuard) -> Result<(), LifecycleError> {
    let output = tmux_output(&["list-panes", "-a", "-F", PANE_FORMAT])?;
    let all = processes()?;
    let mut windows = BTreeMap::<&str, Vec<Vec<&str>>>::new();
    let mut seen = std::collections::HashSet::new();
    for line in output.lines().filter(|line| !line.is_empty()) {
        let fields = line.split(SEPARATOR).collect::<Vec<_>>();
        if fields.len() == 17 && seen.insert(fields[1]) {
            windows.entry(fields[0]).or_default().push(fields);
        }
    }
    for (window, panes) in windows {
        let mut candidates = Vec::new();
        let mut ambiguous = false;
        for f in &panes {
            let agents: Vec<_> = all
                .iter()
                .filter(|p| p.agent.is_some() && belongs_to(p, f[2], &all))
                .collect();
            if agents.len() > 1 {
                ambiguous = true;
            }
            let live = if f[4] == "0" && agents.len() == 1 {
                agents.first().copied()
            } else {
                None
            };
            let retained = f[7].split(':').next() == Some(f[2]) && !f[7].is_empty();
            let launched = f[1] == f[13] && f[2] == f[14];
            if let Some(process) = live {
                let identity = format!("{}:{}:{}", f[2], process.pid, process.birth);
                if f[7] != identity {
                    pane_state(
                        guard,
                        f[1],
                        Lifecycle::Running,
                        "process",
                        "",
                        Some((&identity, process.agent.unwrap().command())),
                    )?;
                }
                candidates.push((f, "running"));
            } else if f[4] == "1"
                && (retained || launched || AgentKind::from_command(f[3]).is_some())
            {
                // A surviving dead pane is tmux's authoritative exit receipt.
                // Keep a hook handoff visible alongside its separate exit.
                if !matches!(f[9], "done" | "needs_input" | "failed") || !retained {
                    let state = if (!f[5].is_empty() && f[5] != "0") || !f[15].is_empty() {
                        Lifecycle::Failed
                    } else {
                        Lifecycle::Unknown
                    };
                    let identity = format!("{}:exited", f[2]);
                    let agent = AgentKind::from_command(f[3])
                        .map(|a| a.command())
                        .unwrap_or("");
                    pane_state(
                        guard,
                        f[1],
                        state,
                        "process",
                        if retained { f[9] } else { "" },
                        (!retained).then_some((&identity, agent)),
                    )?;
                }
                candidates.push((f, "exited"));
            } else if launched && f[4] == "0" && f[7].ends_with(":launch") {
                let state = if f[16] == "starting" {
                    Lifecycle::Starting
                } else {
                    Lifecycle::Running
                };
                pane_state(
                    guard,
                    f[1],
                    state,
                    if f[16] == "starting" {
                        "launch"
                    } else {
                        "process"
                    },
                    f[9],
                    None,
                )?;
                candidates.push((
                    f,
                    if f[16] == "starting" {
                        "starting"
                    } else {
                        "running"
                    },
                ));
            } else if agents.len() <= 1 {
                pane_options(
                    guard,
                    f[1],
                    &[
                        ("identity", ""),
                        ("agent", ""),
                        ("state", ""),
                        ("source", ""),
                        ("since", ""),
                        ("attention_since", ""),
                    ],
                )?;
            }
        }
        // Multiple independent agents have no window-level owner. Do not let
        // pane order or the active pane choose which worker gets represented.
        if ambiguous || candidates.len() > 1 {
            clear(guard, window)?;
            set_option(guard, window, "@drudwyn_state", "unknown")?;
            set_option(guard, window, "@drudwyn_process", "ambiguous")?;
            continue;
        }
        let Some((f, process_state)) = candidates.first() else {
            clear(guard, window)?;
            continue;
        };
        for key in ["agent", "state", "source", "since", "attention_since"] {
            let value = tmux_output(&[
                "show-option",
                "-pqv",
                "-t",
                f[1],
                &format!("@drudwyn_p_{key}"),
            ])?;
            set_option(guard, window, &format!("@drudwyn_{key}"), &value)?;
        }
        set_option(guard, window, "@drudwyn_activity_pane", f[1])?;
        set_option(guard, window, "@drudwyn_process", process_state)?;
        set_option(
            guard,
            window,
            "@drudwyn_exit_code",
            if *process_state == "exited" { f[5] } else { "" },
        )?;
        set_option(
            guard,
            window,
            "@drudwyn_exit_time",
            if *process_state == "exited" { f[6] } else { "" },
        )?;
        set_option(
            guard,
            window,
            "@drudwyn_exit_signal",
            if *process_state == "exited" {
                f[15]
            } else {
                ""
            },
        )?;
        set_option(guard, window, "@drudwyn_message", "")?;
        let state = tmux_output(&["show-option", "-wqv", "-t", window, "@drudwyn_state"])?;
        write_style(guard, window, Lifecycle::from_tmux(&state))?;
    }
    Ok(())
}

pub fn starting(pane: &str, pid: &str, agent: Option<AgentKind>) -> Result<(), LifecycleError> {
    let guard = LifecycleGuard::acquire()?;
    let window = tmux_output(&["display-message", "-p", "-t", pane, "#{window_id}"])?;
    set_option(&guard, &window, "@drudwyn_launch_stage", "starting")?;
    let identity = tmux_output(&["show-option", "-pqv", "-t", pane, "@drudwyn_p_identity"])?;
    // A worker may report a hook before new-window returns. Never overwrite
    // that stronger evidence with the coordinator's startup bookkeeping.
    if identity.split(':').next() != Some(pid) {
        pane_state(
            &guard,
            pane,
            Lifecycle::Starting,
            "launch",
            "",
            Some((
                &format!("{pid}:launch"),
                agent.map(|a| a.command()).unwrap_or(""),
            )),
        )?;
    }
    reconcile(&guard)
}

pub fn hook(agent: AgentKind, event: &str) -> Result<(), LifecycleError> {
    let lifecycle = map_event(agent, event)
        .ok_or_else(|| LifecycleError::UnsupportedEvent(event.to_owned()))?;
    let pane = env::var("TMUX_PANE").map_err(|_| LifecycleError::MissingPane)?;
    // Bind before contention can replace the worker. This is an observation of
    // the pane's unique agent, not proof of the caller's identity or event time.
    let observed = observe_hook_target(&pane, agent)?;
    let guard = LifecycleGuard::acquire()?;
    let current = observe_hook_target(&pane, agent)?;
    if observed.identity != current.identity {
        return Err(LifecycleError::ChangedAgent);
    }
    pane_state(
        &guard,
        &pane,
        lifecycle,
        "hook",
        &current.previous_state,
        Some((&current.identity, agent.command())),
    )?;
    reconcile(&guard)
}

struct HookTarget {
    identity: String,
    previous_state: String,
}

fn observe_hook_target(pane: &str, agent: AgentKind) -> Result<HookTarget, LifecycleError> {
    let record = tmux_output(&["display-message", "-p", "-t", pane, PANE_FORMAT])?;
    let f: Vec<_> = record.split(SEPARATOR).collect();
    if f.len() != 17 || f[1] != pane || f[4] != "0" {
        return Err(LifecycleError::Unattributed);
    }
    let all = processes()?;
    let agents: Vec<_> = all
        .iter()
        .filter(|p| p.agent.is_some() && belongs_to(p, f[2], &all))
        .collect();
    if agents.len() != 1 || agents[0].agent != Some(agent) {
        return Err(LifecycleError::Unattributed);
    }
    let identity = format!("{}:{}:{}", f[2], agents[0].pid, agents[0].birth);
    Ok(HookTarget {
        previous_state: if f[7] == identity { f[9] } else { "" }.to_owned(),
        identity,
    })
}

fn pane_options(
    guard: &LifecycleGuard,
    pane: &str,
    options: &[(&str, &str)],
) -> Result<(), LifecycleError> {
    // Submit state and its binding in one synchronous tmux command queue,
    // with identity last. A failed submission must not publish a new identity
    // before replacing the old process's evidence.
    let names: Vec<_> = options
        .iter()
        .map(|(name, _)| format!("@drudwyn_p_{name}"))
        .collect();
    let mut args = Vec::new();
    for ((_, value), name) in options.iter().zip(&names) {
        if !args.is_empty() {
            args.push(";");
        }
        args.extend(["set-option", "-pq", "-t", pane, name, value]);
    }
    tmux_status(guard, &args)
}
fn pane_state(
    guard: &LifecycleGuard,
    pane: &str,
    lifecycle: Lifecycle,
    source: &str,
    previous: &str,
    binding: Option<(&str, &str)>,
) -> Result<(), LifecycleError> {
    let state = match lifecycle {
        Lifecycle::Starting => "starting",
        Lifecycle::Running => "running",
        Lifecycle::Working => "working",
        Lifecycle::Waiting => "needs_input",
        Lifecycle::Review => "done",
        Lifecycle::Failed => "failed",
        Lifecycle::Unknown => "unknown",
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string();
    let mut options = Vec::new();
    if previous != state {
        options.push(("since", now.as_str()));
        options.push((
            "attention_since",
            if lifecycle.needs_attention() {
                &now
            } else {
                ""
            },
        ));
    }
    options.extend([("state", state), ("source", source)]);
    if let Some((identity, agent)) = binding {
        options.extend([("agent", agent), ("identity", identity)]);
    }
    pane_options(guard, pane, &options)
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

/// Repaint existing markers without altering lifecycle evidence or timestamps.
pub fn refresh_styles() -> Result<(), LifecycleError> {
    let guard = LifecycleGuard::acquire()?;
    let windows = tmux_output(&["list-windows", "-a", "-F", "#{window_id}␟#{@drudwyn_state}"])?;
    for line in windows.lines() {
        if let Some((window, state)) = line.split_once(SEPARATOR) {
            if !state.is_empty() {
                write_style(&guard, window, Lifecycle::from_tmux(state))?;
            }
        }
    }
    Ok(())
}

fn write_style(
    guard: &LifecycleGuard,
    window_id: &str,
    lifecycle: Lifecycle,
) -> Result<(), LifecycleError> {
    let theme = tmux_output(&["show-option", "-gqv", "@drudwyn-theme"])?;
    let dawn = theme == "dawn";
    let (name, fallback) = match lifecycle {
        Lifecycle::Running | Lifecycle::Working | Lifecycle::Starting => {
            ("working", if dawn { "#56949f" } else { "#9ccfd8" })
        }
        Lifecycle::Waiting => ("needs-input", if dawn { "#ea9d34" } else { "#f6c177" }),
        Lifecycle::Review => (
            "done",
            if dawn {
                "#286983"
            } else if theme == "rose-pine" {
                "#31748f"
            } else {
                "#3e8fb0"
            },
        ),
        Lifecycle::Failed => ("failed", if dawn { "#b4637a" } else { "#eb6f92" }),
        Lifecycle::Unknown => {
            set_option(guard, window_id, "@drudwyn_marker", "")?;
            return set_option(guard, window_id, "@drudwyn_window_style", "");
        }
    };
    let configured_color =
        tmux_output(&["show-option", "-gqv", &format!("@drudwyn-{name}-color")])?;
    let color = if configured_color.is_empty() || configured_color == "default" {
        fallback
    } else {
        &configured_color
    };
    let configured_symbol =
        tmux_output(&["show-option", "-gqv", &format!("@drudwyn-{name}-symbol")])?;
    let symbol = if configured_symbol.is_empty() {
        "●"
    } else {
        &configured_symbol
    };
    let marker = format!("#[fg={color}]{symbol}#[default] ");
    set_option(guard, window_id, "@drudwyn_marker", &marker)?;
    set_option(
        guard,
        window_id,
        "@drudwyn_window_style",
        &format!("#[fg={color}]"),
    )?;
    Ok(())
}

fn clear(guard: &LifecycleGuard, window_id: &str) -> Result<(), LifecycleError> {
    for option in [
        "@drudwyn_state",
        "@drudwyn_source",
        "@drudwyn_message",
        "@drudwyn_since",
        "@drudwyn_attention_since",
        "@drudwyn_marker",
        "@drudwyn_window_style",
        "@drudwyn_agent",
        "@drudwyn_activity_pane",
        "@drudwyn_process",
        "@drudwyn_exit_code",
        "@drudwyn_exit_time",
        "@drudwyn_exit_signal",
    ] {
        set_option(guard, window_id, option, "")?;
    }
    Ok(())
}

fn set_option(
    guard: &LifecycleGuard,
    window_id: &str,
    name: &str,
    value: &str,
) -> Result<(), LifecycleError> {
    tmux_status(guard, &["set-option", "-wq", "-t", window_id, name, value])
}

fn tmux_output(args: &[&str]) -> Result<String, LifecycleError> {
    let output = Command::new("tmux").args(args).output()?;
    if !output.status.success() {
        return Err(LifecycleError::Tmux);
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn tmux_status(guard: &LifecycleGuard, args: &[&str]) -> Result<(), LifecycleError> {
    // set-option never consumes stdin. Stdio safely duplicates the same open
    // file description into this child only, so it shares the kernel lock even
    // if our process is killed. Reads and unrelated subprocesses do not inherit
    // it. Do not explicitly unlock: the last surviving owner releases the lock.
    Command::new("tmux")
        .args(args)
        .stdin(Stdio::from(guard.directory.try_clone()?))
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
