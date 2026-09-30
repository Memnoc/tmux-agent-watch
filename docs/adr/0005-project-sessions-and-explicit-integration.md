---
status: accepted
date: 2026-09-30
proposed-by: Codex
approved-by: Memnoc
---

# ADR-0005: Share a project session and integrate workers explicitly

## Context

The [worktree field report](../intake/2026-09-30-worktree-worker-workflow.md)
describes hard-to-identify workers, repeated loss of the planning window from
view, manual merge commands, and unclear recovery after an agent exits.
The workflow needs a recognizable coordination location while preserving
[ADR-0002's stateless boundary](0002-rust-stateless-v2-architecture.md).

## Decision

A project uses a clearly identified shared tmux session with a coordinator
workspace and named worker windows; Drudwyn also exposes a New Session action
for ad hoc shells and editing.
Users initiate integration through an explicit Integrate action after review,
choosing either direct integration into the configured base branch (normally
`main`) or creation of an integration branch where workers' changes can be
assembled and verified first.
Task associations remain live tmux metadata, with task files owned by the user's
repository and no Drudwyn prompt history or durable task registry.

## Considered options

- **Shared project session, explicit integration, live associations** — chosen
  to keep related work recognizable, make merging intentional, and preserve the
  existing stateless design.
- **A session for every worker** — not chosen as the default; users still need
  an independent New Session action for their other terminal work.
- **Automatic integration on worker completion** — not chosen; completion is a
  signal to review, not an instruction to merge.
- **Always merge into the base or always require an integration branch** —
  neither is imposed; the user chooses the workflow for the work being started.
- **Durable task associations across tmux restarts** — deferred; Memnoc prefers
  retaining the stateless design until experience shows a need to change it.

## Consequences

The session, coordinator, and worker roles must be visible in navigation.
Independent navigation in separate terminals remains required; how tmux client
views implement that requirement is not settled by this decision.
After live task metadata is lost, a user reselects a repository task file or
provides a fresh task; reopening a worktree does not restore a conversation.
The coordinator agent handles integration conflicts, with Continue/Abort
actions in Drudwyn; completing a merge remains separate from verification of
the assembled result. Enforced port reservations are deferred until after the
core workflow.

The destination choice is presented when setting up the work, kept visible,
and retained only in live tmux metadata for that batch; it is not requested
again for each sibling worker. Creating an integration branch includes choosing
its name and inspecting its starting ref/commit, and must preserve existing
checkouts and branches. Final integration from that branch into the base is a
separate explicit action. Neither path implicitly pushes or deploys.

This records the agreed design direction, not shipped application behavior.
