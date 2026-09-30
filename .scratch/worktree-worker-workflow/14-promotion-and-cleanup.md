# 14 — Promote integration branches and clean up safely

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** ready

**What to build:** Users explicitly merge an integration branch into the base,
then remove eligible worker worktrees without losing branches or active work.

**Blocked by:** 12, 13.
**Priority:** P1. **Stories:** 20, 21.

- [ ] Offer explicit promotion with a fresh source/target preview using the
  same merge/conflict/verification controls; never auto-promote on worker completion.
- [ ] Verification on the integration branch is not copied as proof of a new
  base merge; expose the base destination's own verification state.
- [ ] Finish requires a clean worktree, containment in the chosen destination,
  and no unresolved Git operation or active writer; unrelated primary HEAD
  ancestry alone is insufficient.
- [ ] Confirm removal and preserve the branch; close only windows for the removed
  worktree and preserve the coordinator and other linked windows/sessions.
- [ ] Dirty/untracked/unmerged work, detached checkouts, moving branch tips,
  removal failures, and repeated requests remain safe and understandable.
- [ ] Neither route pushes, deploys, deletes branches, or implies shipment.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.
