# Domain Context

## Terms

### Workspace

An active tmux window that hosts an agent or shell in a Git checkout. A workspace may use the primary checkout or a linked worktree.

### Linked worktree

An additional Git working directory attached to the same repository and checked out on its own branch. It isolates files and Git state, but not operating-system resources such as ports or databases.

### Workspace Cockpit

The tmux-native, action-first popup used to start, review, jump to, and finish workspaces. It derives live state from Git and tmux rather than owning a persistent registry.

### Ambient status

The always-visible sidebar and HUD that show which workspaces need attention. Ambient status reports state; the Workspace Cockpit performs lifecycle actions.

### Content-blind supervision

Deterministic observation of non-content tmux, process, agent-lifecycle, and Git metadata. It never reads terminal scrollback or retains prompts, responses, permission details, or derived summaries.

### Live operational metadata

Non-content state needed to operate the current tmux session: workspace identifiers, agent kind, lifecycle state, evidence source, timestamps, repository/worktree path, branch, and Git readiness flags. The project does not persist it across sessions.

### Review

Returning to the authoritative agent workspace to inspect and continue the work. Review is tool-agnostic; tmux-agent-watch does not prescribe a Git user interface.
