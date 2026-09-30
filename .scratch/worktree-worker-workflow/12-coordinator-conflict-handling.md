# 12 — Resolve integration conflicts through the coordinator

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** ready

**What to build:** A conflicting integration has a visible agent handoff and
Continue/Abort controls in Drudwyn.

**Blocked by:** 11.
**Priority:** P1. **Stories:** 18, 21, 22.

- [ ] Create a controlled merge conflict and leave it in the correct destination
  checkout with source/target identity and an explicit conflict state.
- [ ] Route a transient instruction to the coordinator agent in that checkout;
  expose Open/recovery/retry when it is absent or delivery is uncertain.
- [ ] Continue requires resolved unmerged entries and the expected operation;
  it cannot complete an unrelated merge or imply that assembled checks passed.
- [ ] Abort uses Git's normal behavior and preserves work on failure; do not use
  hard reset or silently discard resolution edits as a fallback.
- [ ] Reconcile if the coordinator completed/aborted independently; repeated
  actions do not duplicate commits or resend tasks without deliberate retry.
- [ ] Drudwyn inspects Git metadata only; conflict contents and resolution are
  handled by the coordinator agent, not terminal scraping or a hidden merge solver.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.
