# Worktree worker implementation tickets

Approved by Memnoc on 2026-09-30.

[Specification](../../docs/specs/2026-09-30-worktree-worker-workflow.md) ·
[Approved breakdown](../../docs/specs/2026-09-30-worktree-worker-ticket-plan.md)

Work any ready ticket whose blockers are all done. Initial frontier: 01 and 02.
Implementation branch: `work/worktree-worker-workflow`.

- [01 — Preserve work on launch failure](01-preserve-work-on-launch-failure.md)
- [02 — Navigate independently in two terminals](02-independent-terminal-navigation.md)
- [03 — Identify and return to the coordinator](03-coordinator-identity.md)
- [04 — Create a shell session from Drudwyn](04-new-session.md)
- [05 — Choose the batch source and integration destination](05-batch-source-and-destination.md)
- [06 — Launch a named worker with a complete task](06-worker-launch-form.md)
- [07 — Show evidence-backed activity and exit state](07-activity-and-exit-state.md)
- [08 — Recover a worktree and deliberately restart its task](08-worktree-recovery.md)
- [09 — Supervise workers in a global Cockpit](09-global-cockpit.md)
- [10 — Ship status-bar A with density controls](10-status-bar-a.md)
- [11 — Integrate reviewed worker commits](11-integrate-worker-commits.md)
- [12 — Resolve integration conflicts through the coordinator](12-coordinator-conflict-handling.md)
- [13 — Verify the assembled checkout](13-assembled-verification.md)
- [14 — Promote integration branches and clean up safely](14-promotion-and-cleanup.md)
- [15 — Harden the assembled workflow](15-assembled-hardening.md)
