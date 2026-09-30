use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentKind {
    Codex,
    Claude,
    OpenCode,
    Unknown,
}

impl AgentKind {
    pub fn from_command(command: &str) -> Option<Self> {
        let executable = command.rsplit('/').next().unwrap_or(command);
        match executable {
            "codex" => Some(Self::Codex),
            "claude" => Some(Self::Claude),
            "opencode" => Some(Self::OpenCode),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Codex => "Codex",
            Self::Claude => "Claude",
            Self::OpenCode => "OpenCode",
            Self::Unknown => "Agent",
        }
    }

    pub fn command(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
            Self::OpenCode => "opencode",
            Self::Unknown => "codex",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Lifecycle {
    Starting,
    Working,
    Running,
    Waiting,
    Review,
    Failed,
    #[default]
    Unknown,
}

impl Lifecycle {
    pub fn from_tmux(value: &str) -> Self {
        match value {
            "starting" => Self::Starting,
            "working" => Self::Working,
            "running" => Self::Running,
            "needs_input" | "waiting" => Self::Waiting,
            "done" | "review" => Self::Review,
            "failed" => Self::Failed,
            _ => Self::Unknown,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Starting => "STARTING",
            Self::Working => "WORKING",
            Self::Running => "RUNNING",
            Self::Waiting => "NEEDS INPUT",
            Self::Review => "REVIEW",
            Self::Failed => "FAILED",
            Self::Unknown => "UNKNOWN",
        }
    }

    pub fn needs_attention(self) -> bool {
        matches!(self, Self::Waiting | Self::Review | Self::Failed)
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum EvidenceSource {
    Hook,
    Process,
    Launch,
    #[default]
    Unknown,
}

impl EvidenceSource {
    pub fn from_tmux(value: &str) -> Self {
        match value {
            "hook" => Self::Hook,
            "launch" => Self::Launch,
            "observer" | "process" => Self::Process,
            _ => Self::Unknown,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Hook => "hook",
            Self::Process => "process",
            Self::Launch => "launch",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum GitState {
    Clean,
    Dirty,
    #[default]
    Unknown,
}

impl GitState {
    pub fn from_tmux(value: &str) -> Self {
        match value {
            "clean" => Self::Clean,
            "dirty" => Self::Dirty,
            _ => Self::Unknown,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkspaceIdentity {
    pub session: String,
    /// All live session memberships, including independent terminal views.
    pub sessions: Vec<String>,
    pub window_id: String,
    pub window_name: String,
    pub pane_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Checkout {
    pub working_directory: PathBuf,
    pub repository: Option<PathBuf>,
    pub worktree: Option<PathBuf>,
    pub branch: Option<String>,
    pub git_state: GitState,
    pub is_linked_worktree: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Workspace {
    pub identity: WorkspaceIdentity,
    pub checkout: Checkout,
    pub project: Option<String>,
    pub coordinator: Option<String>,
    pub coordinator_available: bool,
    pub agent: AgentKind,
    pub lifecycle: Lifecycle,
    pub evidence: EvidenceSource,
    pub state_since: Option<u64>,
    pub attention_since: Option<u64>,
    pub process: String,
    pub exit_code: Option<i32>,
    pub exit_signal: Option<String>,
    pub exit_time: Option<u64>,
}

impl Workspace {
    pub fn process_label(&self) -> String {
        match self.process.as_str() {
            "running" => "Running process".into(),
            "starting" => "Launch in progress".into(),
            "ambiguous" => "Ambiguous: multiple agents; ownership unknown".into(),
            "exited" => format!(
                "Exited{} · exit code {} · signal {} · time {} (not task completion)",
                if self.exit_code.is_some_and(|c| c != 0) || self.exit_signal.is_some() {
                    " (FAILED)"
                } else {
                    ""
                },
                self.exit_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "unknown".into()),
                self.exit_signal.as_deref().unwrap_or("none/unknown"),
                self.exit_time
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| "unknown".into())
            ),
            _ => "Process/exit unknown".into(),
        }
    }
    pub fn is_agent(&self) -> bool {
        self.agent != AgentKind::Unknown
            || self.lifecycle != Lifecycle::Unknown
            || !self.process.is_empty()
    }

    pub fn role(&self) -> &'static str {
        if self.coordinator.as_deref() == Some(self.identity.window_id.as_str()) {
            if self.is_agent() {
                "Coordinator agent"
            } else {
                "Coordinator shell"
            }
        } else if self.checkout.is_linked_worktree {
            "Worktree worker"
        } else if self.is_agent() {
            "Ordinary agent"
        } else {
            "Shell"
        }
    }

    pub fn sort_key(&self) -> (u8, u64, &str, &str) {
        let group = if self.lifecycle.needs_attention() {
            0
        } else {
            1
        };
        let since = self
            .attention_since
            .or(self.state_since)
            .unwrap_or(u64::MAX);
        (
            group,
            since,
            &self.identity.session,
            &self.identity.window_id,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_v1_states_without_content() {
        assert_eq!(Lifecycle::from_tmux("needs_input"), Lifecycle::Waiting);
        assert_eq!(Lifecycle::from_tmux("done"), Lifecycle::Review);
        assert_eq!(Lifecycle::from_tmux("anything-else"), Lifecycle::Unknown);
    }
}
