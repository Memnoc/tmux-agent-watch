---
status: accepted
date: 2026-09-01
proposed-by: Memnoc
approved-by: Memnoc
---

# ADR-0003: Keep workspace review tool-agnostic

## Context

The first cockpit implementation treated lazygit as the review destination.
That added an external dependency and made a workspace lifecycle action depend
on one optional Git interface. The authoritative task context already lives in
the agent pane, while users may prefer command-line Git, an editor, another Git
UI, or agent-assisted review.

## Decision

Review jumps directly to the selected agent workspace. The project does not
install, detect, configure, or bind lazygit, and it does not prescribe a Git
review interface. Git operations required for workspace safety remain typed,
guarded commands owned by the Rust binary.

## Consequences

The `g` binding and `@agent-watch-lazygit-key` option are removed. Users retain
their existing tmux, shell, editor, and Git tools inside the selected workspace.
The cockpit stays focused on navigation and workspace lifecycle orchestration.
