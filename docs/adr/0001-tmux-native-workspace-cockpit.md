---
status: accepted
date: 2026-09-01
proposed-by: Memnoc
approved-by: Memnoc
---

# ADR-0001: Add a tmux-native workspace cockpit

## Context

Individual shortcuts made the Git worktree workflow functional but difficult to discover and remember. A browser spike compared a lifecycle sidebar, an action-first command center, and a fleet board; the action-first command center provided the clearest path without replacing tmux.

## Decision

tmux remains the process, navigation, and persistence host, while a Bash Workspace Cockpit opened with `prefix + P` becomes the primary interface for starting, reviewing, jumping to, and finishing workspaces. The cockpit derives state from Git and tmux, keeps existing shortcuts as expert paths, reports failures inline, and does not install dependencies or start development servers in its first version.

## Considered options

- **tmux-native action-first cockpit** — chosen because it unifies the workflow without adding a runtime, daemon, database, or second source of truth.
- **Enhanced persistent sidebar** — rejected as the primary interaction because it improves visibility but still spreads actions across memorized shortcuts.
- **Fleet-board interface** — rejected as the primary interaction because it emphasizes monitoring over the next action.
- **Standalone workspace application** — rejected because it would duplicate tmux process and navigation responsibilities.

## Consequences

The sidebar remains the ambient status surface, and direct `W`, `g`, and `X` bindings remain available. Repository-owned setup commands and development-server orchestration require later decisions; the first cockpit release performs no implicit project commands.
