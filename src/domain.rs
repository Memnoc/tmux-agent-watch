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
            Self::Waiting => "WAITING",
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
    #[default]
    Unknown,
}

impl EvidenceSource {
    pub fn from_tmux(value: &str) -> Self {
        match value {
            "hook" => Self::Hook,
            "observer" | "process" => Self::Process,
            _ => Self::Unknown,
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
    pub agent: AgentKind,
    pub lifecycle: Lifecycle,
    pub evidence: EvidenceSource,
    pub state_since: Option<u64>,
    pub attention_since: Option<u64>,
}

impl Workspace {
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
