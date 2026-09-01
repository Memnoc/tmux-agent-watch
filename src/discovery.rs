use std::{path::PathBuf, process::Command};

use thiserror::Error;

use crate::domain::{
    AgentKind, Checkout, EvidenceSource, GitState, Lifecycle, Workspace, WorkspaceIdentity,
};

// Tmux truncates format strings containing ASCII control separators. This
// printable Unicode symbol survives argv and tmux formatting unchanged.
const FIELD_SEPARATOR: char = '\u{241f}';

// Intentionally excludes @agent_watch_message and pane content. This format is
// part of the project's content-blind privacy boundary.
const WINDOW_FORMAT: &str = "#{session_name}␟#{window_id}␟#{window_name}␟#{pane_id}␟#{pane_current_path}␟#{pane_current_command}␟#{@agent_watch_state}␟#{@agent_watch_source}␟#{@agent_watch_since}␟#{@agent_watch_attention_since}␟#{@agent_watch_repo}␟#{@agent_watch_worktree}␟#{@agent_watch_branch}␟#{@agent_watch_git_status}";

#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("could not invoke tmux: {0}")]
    Spawn(#[from] std::io::Error),
    #[error("tmux discovery failed: {0}")]
    Tmux(String),
    #[error("invalid tmux record: expected 14 fields, found {found}")]
    Record { found: usize },
}

pub fn discover() -> Result<Vec<Workspace>, DiscoveryError> {
    let mut workspaces = discover_tmux()?;
    for workspace in &mut workspaces {
        reconcile_git(workspace);
    }
    workspaces.sort_by(|left, right| left.sort_key().cmp(&right.sort_key()));
    Ok(workspaces)
}

pub fn discover_tmux() -> Result<Vec<Workspace>, DiscoveryError> {
    let output = Command::new("tmux")
        .args(["list-windows", "-a", "-F", WINDOW_FORMAT])
        .output()?;
    if !output.status.success() {
        return Err(DiscoveryError::Tmux(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    parse_windows(&String::from_utf8_lossy(&output.stdout))
}

pub fn parse_windows(input: &str) -> Result<Vec<Workspace>, DiscoveryError> {
    let mut workspaces = input
        .lines()
        .filter(|line| !line.is_empty())
        .map(parse_window)
        .collect::<Result<Vec<_>, _>>()?;
    workspaces.retain(|workspace| {
        workspace.agent != AgentKind::Unknown || workspace.lifecycle != Lifecycle::Unknown
    });
    workspaces.sort_by(|left, right| left.sort_key().cmp(&right.sort_key()));
    Ok(workspaces)
}

fn parse_window(line: &str) -> Result<Workspace, DiscoveryError> {
    let fields = line.split(FIELD_SEPARATOR).collect::<Vec<_>>();
    if fields.len() != 14 {
        return Err(DiscoveryError::Record {
            found: fields.len(),
        });
    }
    let path = PathBuf::from(fields[4]);
    Ok(Workspace {
        identity: WorkspaceIdentity {
            session: fields[0].into(),
            window_id: fields[1].into(),
            window_name: fields[2].into(),
            pane_id: fields[3].into(),
        },
        checkout: Checkout {
            working_directory: path,
            repository: optional_path(fields[10]),
            worktree: optional_path(fields[11]),
            branch: optional_string(fields[12]),
            git_state: GitState::from_tmux(fields[13]),
            is_linked_worktree: !fields[11].is_empty(),
        },
        agent: AgentKind::from_command(fields[5]).unwrap_or(AgentKind::Unknown),
        lifecycle: Lifecycle::from_tmux(fields[6]),
        evidence: EvidenceSource::from_tmux(fields[7]),
        state_since: fields[8].parse().ok(),
        attention_since: fields[9].parse().ok(),
    })
}

fn optional_string(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

fn optional_path(value: &str) -> Option<PathBuf> {
    (!value.is_empty()).then(|| PathBuf::from(value))
}

fn reconcile_git(workspace: &mut Workspace) {
    let path = &workspace.checkout.working_directory;
    let Some(top) = git(path, &["rev-parse", "--show-toplevel"]) else {
        return;
    };
    let Some(common_dir) = git(
        path,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    ) else {
        return;
    };
    let Some(git_dir) = git(path, &["rev-parse", "--path-format=absolute", "--git-dir"]) else {
        return;
    };

    workspace.checkout.repository = Some(PathBuf::from(top.trim()));
    workspace.checkout.branch =
        git(path, &["branch", "--show-current"]).and_then(|branch| optional_string(branch.trim()));
    workspace.checkout.git_state =
        match git(path, &["status", "--porcelain", "--untracked-files=normal"]) {
            Some(status) if status.is_empty() => GitState::Clean,
            Some(_) => GitState::Dirty,
            None => GitState::Unknown,
        };
    workspace.checkout.is_linked_worktree = common_dir.trim() != git_dir.trim();
    workspace.checkout.worktree = workspace
        .checkout
        .is_linked_worktree
        .then(|| PathBuf::from(top.trim()));
}

fn git(path: &std::path::Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .ok()?;
    output.status.success().then(|| {
        String::from_utf8_lossy(&output.stdout)
            .trim_end()
            .to_owned()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_prioritises_attention_without_message_content() {
        let sep = FIELD_SEPARATOR;
        let input = format!(
            "dev{sep}@2{sep}api{sep}%2{sep}/repo-wt{sep}codex{sep}working{sep}hook{sep}20{sep}{sep}/repo{sep}/repo-wt{sep}work/api{sep}dirty\n\
             dev{sep}@1{sep}ui{sep}%1{sep}/repo{sep}claude{sep}needs_input{sep}hook{sep}10{sep}11{sep}/repo{sep}{sep}main{sep}clean\n"
        );
        let result = parse_windows(&input).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].identity.window_id, "@1");
        assert_eq!(result[0].lifecycle, Lifecycle::Waiting);
        assert_eq!(result[1].checkout.git_state, GitState::Dirty);
        assert_eq!(result[1].agent, AgentKind::Codex);
    }

    #[test]
    fn ignores_unrecognised_shell_windows() {
        let sep = FIELD_SEPARATOR;
        let input = format!(
            "dev{sep}@1{sep}shell{sep}%1{sep}/repo{sep}{sep}{sep}{sep}{sep}{sep}{sep}{sep}{sep}\n"
        );
        assert!(parse_windows(&input).unwrap().is_empty());
    }

    #[test]
    fn ignores_non_agent_processes() {
        let sep = FIELD_SEPARATOR;
        let input = format!(
            "dev{sep}@1{sep}editor{sep}%1{sep}/repo{sep}nvim{sep}{sep}{sep}{sep}{sep}{sep}{sep}{sep}\n"
        );
        assert!(parse_windows(&input).unwrap().is_empty());
    }

    #[test]
    fn tmux_discovery_format_is_content_blind() {
        for forbidden in [
            "capture-pane",
            "@agent_watch_message",
            "pane_history",
            "pane_title",
            "command_arguments",
        ] {
            assert!(!WINDOW_FORMAT.contains(forbidden), "requested {forbidden}");
        }
        assert!(WINDOW_FORMAT.contains("@agent_watch_state"));
        assert!(WINDOW_FORMAT.contains("pane_current_path"));
    }
}
