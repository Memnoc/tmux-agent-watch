# 13 — Verify the assembled checkout

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** ready

**What to build:** Users launch explicitly selected checks in the integration
checkout and see truthful revision-specific results separate from worker reports.

**Blocked by:** 11.
**Priority:** P1. **Stories:** 19, 21, 22.

- [ ] Offer a visible verification action for user/coordinator-selected checks
  in the target checkout; do not guess commands or execute automatically on merge.
- [ ] Associate live results with check identity, checkout, tested revision,
  timestamps, and exit status, without retaining command output or task content.
- [ ] Distinguish reported worker checks, Not verified, Running, Passed, Failed,
  and stale/missing evidence; worker Review never manufactures a check result.
- [ ] Failed/interrupted checks leave integrated work intact and allow rerun;
  a revision/observed working-tree change invalidates the current-result claim.
- [ ] Include an untracked application input that makes assembled checks fail
  despite clean worker branches; show the tested environment's limits accurately.
- [ ] Losing tmux metadata resets unprovable verification status, while Git
  ancestry can still establish integration independently.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.
