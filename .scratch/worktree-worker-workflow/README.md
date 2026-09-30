# Worktree worker implementation tickets

Approved by Memnoc on 2026-09-30.

[Specification](../../docs/specs/2026-09-30-worktree-worker-workflow.md) ·
[Approved breakdown](../../docs/specs/2026-09-30-worktree-worker-ticket-plan.md)

Work any ready ticket whose blockers are all done when the user resumes work.
Implementation branch: `work/worktree-worker-workflow`.

## Pause checkpoint — after ticket 11

Paused at the user's request after ticket11 independent clearance. Tickets01–11
are done; reviewed implementation HEAD is
`346e42340a1b5c79c3ac586178326c742681c919` (the documentation receipt follows it).
Next is ticket12, then13/14/15 in dependency order. Do not start further work
until the user resumes. Reviewer `rereview07d` performed read-only ticket12
preparation only; ticket12 implementation has not started.

Continue on `work/worktree-worker-workflow`; `main` remains
`eaf24469290cbf77dd1d2a6176fbd54f7ace1868`. No push was performed.
The frozen full suite passed (41 Rust tests, integration16/activity39 and all
other gates); see ticket11's final receipts for exact logs and independent
checks. Runtime validation is Linux/tmux3.4; other platforms are unverified.
The existing optional Resurrect test is skipped. Shell activity empty-text and
singular-project wording remain cosmetic ticket15 work.

Existing seams for resumed conflict work: `integration::preview/apply` owns
reviewed refs, canonical destinations, locking and preserved merge outcomes;
`recovery::selected_checkout` resolves stable pane checkout identity;
`workspace::deliver`/`deliver_task`/`deliver_reference` own transient delivery
and its checkout guard. `workspace::checkout_git` isolates explicit checkout
commands from inherited Git addressing. Reuse these boundaries after reading
ticket12/spec/ADRs; do not infer verification, cleanup or conflict resolution
from integration success.

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
