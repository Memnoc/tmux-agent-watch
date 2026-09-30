# 11 — Integrate reviewed worker commits

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** ready

**What to build:** Users inspect and integrate commits into the chosen destination
from Cockpit or a command, without manually opening a shell to run Git.

**Blocked by:** 05, 09.
**Priority:** P1. **Stories:** 17, 21, 22.

- [ ] Preview source/target refs, commit IDs, target checkout, changed-file
  metadata, and available check evidence; revalidate before applying.
- [ ] Perform fast-forward or normal divergent clean merge; report already-
  contained work as a no-op. Update integration from actual ancestry, not Review.
- [ ] Reject dirty/detached/ambiguous targets, active Git operations, stale
  previews, and unsafe untracked-file collisions without stashing or resetting.
- [ ] Serialize Drudwyn operations on the same destination and preserve its
  existing checkout; a second client cannot concurrently start another merge there.
- [ ] Keep merge failure/conflict visible and recoverable, with no success claim
  or cleanup. Coordinator resolution is added by 12; this slice must remain safe.
- [ ] Test both direct-to-base and integration-branch destinations, missing
  metadata, target-branch checkout elsewhere, cancellation, and Git failures.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.
