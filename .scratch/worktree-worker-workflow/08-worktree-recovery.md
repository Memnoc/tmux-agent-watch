# 08 — Recover a worktree and deliberately restart its task

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** ready

**What to build:** Cockpit exposes surviving worktrees for known repositories
and allows reopening a shell or deliberately restarting an instructed worker.

**Blocked by:** 03, 06, 07.
**Priority:** P1. **Stories:** 7, 8, 11, 15, 22.

- [ ] Enumerate Git worktrees for the selected repository and distinguish those
  without live windows from active workers; include missing coordinator recovery.
- [ ] Reopen the existing branch/path without duplicating a worktree, resetting
  files, or overwriting a live worker; preserve dirty and untracked work.
- [ ] Present Open shell and Restart with task/reference distinctly, explaining
  that a fresh agent does not restore a conversation. Offer resume only when supported.
- [ ] Recover a live task reference where still available, or require explicit
  re-selection after metadata loss; never invent old prompts, checks, or exit history.
- [ ] Report deleted directories, locked/prunable worktrees, missing agents,
  unavailable task files, and racing window creation without destructive repair.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.
