# 01 — Preserve work on launch failure

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** ready

**What to build:** A failed start returns a truthful error and an actionable
path to retained resources instead of removing work the worker may have created.

**Blocked by:** None — can start immediately.
**Priority:** P0. **Stories:** 6, 7, 21, 22.

- [ ] Reproduce immediate zero/nonzero exit, metadata attachment failure, and
  a worker that writes a file or commit before failing through the launch action.
- [ ] Distinguish resources never used by a process from resources it may have
  modified. Retain uncertain, dirty, or newly committed work without force deletion.
- [ ] Display the launch failure and retained worktree/branch identity; do not
  show success because creation alone or a process exit succeeded.
- [ ] Retrying cannot overwrite existing branches/paths or duplicate a surviving
  live worker. Correctable failures leave unrelated windows/checkouts untouched.
- [ ] Verify preservation through real Git state, including untracked files;
  adapt existing rollback expectations only where the spec intentionally changes them.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.
