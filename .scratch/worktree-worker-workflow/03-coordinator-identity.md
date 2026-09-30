# 03 — Identify and return to the coordinator

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** ready

**What to build:** A project has a recognizable coordinator and named windows
with visible roles, with a reliable return action from worker navigation.

**Blocked by:** 02.
**Priority:** P1. **Stories:** 8, 15, 16, 22.

- [ ] Explicitly associate a coordinator workspace with the project context;
  support both a shell and an agent without counting the shell as an active worker.
- [ ] Distinguish linked-worktree worker, ordinary agent, coordinator, and shell
  in navigators and existing Cockpit details using text as well as optional icons.
- [ ] Names derive from a deliberate short name/branch fallback; process title
  changes do not replace managed names, and later user renames are preserved.
- [ ] Starting and switching among three workers preserves the coordinator
  window ID and checkout branch; a vanished coordinator offers explicit recovery.
- [ ] Same-name projects/windows, external unassociated agents, and label
  redaction remain navigable by stable identity.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.
