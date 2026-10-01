# Domain Context

## Terms

### Workspace

An active tmux window that hosts an agent or shell in a Git checkout. A workspace may use the primary checkout or a linked worktree.

### Linked worktree

An additional Git working directory attached to the same repository and checked out on its own branch. It isolates files and Git state, but not operating-system resources such as ports or databases.

### Worker

An agent assigned work in a selected checkout, usually a linked worktree. Its
live terminal is a workspace. A surviving checkout without that terminal is
recoverable work, not evidence that an agent is still running.

### Worker batch

A live association of sibling workers, a coordinator workspace, a pinned
starting commit, and a chosen integration target. The starting point and target
are separate: integrating one sibling does not change another's starting commit.

### Integration target

The branch and checkout selected to receive reviewed worker commits. It may be
the configured base branch or an integration branch, independently of the
checkout from which the workers were launched.

### Project session

The clearly identified shared tmux session containing a project's coordinator
workspace and named worker windows. Additional shell sessions serve ad hoc work
without requiring a new worker or linked worktree.

### Coordinator workspace

The workspace used to plan work, receive worker handoffs, and coordinate
integration within a project session. Its role is distinct from a worker's;
it is not necessarily a checkout of `main`.

### Workspace Cockpit

The tmux-native, action-first popup for global worker supervision across the
connected tmux server: searchable inventory, project and attention filters,
worker details, coordinator access, and workspace lifecycle actions. It derives
live state from Git and tmux rather than owning a persistent registry. Inspecting
a worker is distinct from opening its window. Workspace and session navigators
provide quick movement and session management.

### Ambient status

The always-visible tmux bar defaults to Balanced A: a separator plus one
information row. Current workspace identity/activity anchors the left, selected
branch and Git line counts stay centered, and global attention and agent totals
anchor the right. The bar shows only the current workspace; grouped Workspace
and global Cockpit provide the complete inventory. Focus, Dense and Tabs remain
selectable layouts. Counts and activity are evidence, not inferred task progress.
The former sidebar is an opt-in compatibility surface.

### Content-blind supervision

Deterministic observation of non-content tmux, process, agent-lifecycle, and Git metadata. It never reads terminal scrollback or retains prompts, responses, permission details, or derived summaries.

### Live operational metadata

Non-content state needed to operate the current tmux session: workspace identifiers, agent kind, lifecycle state, evidence source, timestamps, repository/worktree path, branch, and Git readiness flags. The project does not persist it across sessions.

### Review

Returning to the authoritative agent workspace to inspect and continue the work. Review is tool-agnostic; tmux-drudwyn does not prescribe a Git user interface.

### Integrate

A user-initiated action to merge a reviewed worker's commits into a selected
destination branch. Integration is distinct from worker completion, verification
of the assembled result, and removal of the worktree.

### Integration branch

A user-selected branch where worker commits are combined and verified before
being merged into the project's base branch, normally `main`. It is optional;
workers can instead integrate directly into the base branch.

### Assembled verification

Checks run on the checkout containing combined changes, associated with the
tested revision and environment. It is distinct from a worker's reported checks,
an agent's Review state, Git integration, and deployment.
