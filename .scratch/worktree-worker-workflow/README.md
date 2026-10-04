Completed feedback implementation — 2026-10-04: [ticket25](25-live-worktree-test-findings.md)
records all functional and UX fixes, automated receipts and the remaining live
comparison. Managed Codex hook ownership is corrected in ticket19; shared-daemon
sessions are deliberately unsupported until restarted. The installed runtime is
independent of main/feature checkouts. [Repeat procedure](../../docs/specs/2026-10-04-worktree-feedback-comparison.md).

The status entries below are historical checkpoints.

Implementation resumed — 2026-10-04: Memnoc authorized all functional and UX
feedback in [ticket25](25-live-worktree-test-findings.md), including ticket19.
Baseline f530145; verification and independent crosscheck in progress.

Live test follow-up — 2026-10-04: [ticket25](25-live-worktree-test-findings.md)
records runtime path repair, uncertain recovery task delivery and form UX findings.
The hands-on test is still in progress.

Latest checkpoint — 2026-10-04: work preserved on
`work/worktree-worker-workflow` before returning to `main`.
[ADR-0008](../../docs/adr/0008-worktree-worker-workflow-branch-checkpoint.md)
is the consolidated recall record. Last implementation: `5a2c1d1`; tickets23/24
add grouped agents and approved Balanced A status. Ticket19 remains open.
The earlier updates and receipts below are historical.

Latest update — 2026-10-01: the historical pause below was superseded by
Memnoc's resume. Tickets12–15 and follow-up UI implementation are complete.
[Ticket22](22-approved-navigation-layouts.md) records the approved Workspace/
Sessions redesign, restored identity icons and dense six-tab option. Main remains
unchanged; no push or deployment. Earlier receipts below remain historical.

# Worktree worker implementation tickets

Approved by Memnoc on 2026-09-30.

[Specification](../../docs/specs/2026-09-30-worktree-worker-workflow.md) ·
[Approved breakdown](../../docs/specs/2026-09-30-worktree-worker-ticket-plan.md)

All 15 implementation tickets are complete, including assembled Linux acceptance.
Full-auto implementation resumed on 2026-10-01 for tickets 12–15.
Implementation branch: `work/worktree-worker-workflow`.

## Live-use follow-up — 2026-10-01

[16 — Screenshot regressions](16-screenshot-regressions.md) addresses the first
live-use feedback. [17 — Visual polish](17-visual-polish.md) restores branding,
removes raw IDs from overview rows, and fixes crowded labels and controls.
[18 — Popup space and layout proposal](18-popup-space-and-layout-proposal.md)
enlarges the two inventories and provides three reviewable designs.
[19 — Codex hook ownership](19-codex-hook-ownership.md) records a reproduced
live integration failure; its fix remains outstanding.
[20 — Approved Cockpit and one-row proposals](20-approved-cockpit-and-one-row-proposals.md)
implements the selected visual direction and explores a more compact status bar.
The earlier acceptance below covered workflow mechanics but
missed ordinary unregistered sessions and usable inventory space at 84 × 27.

## Completion checkpoint — 2026-10-01

Tickets 01–15 are implemented on `work/worktree-worker-workflow`.
[Ticket15](15-assembled-hardening.md) records the frozen complete suite and
[assembled acceptance report](../../docs/specs/2026-10-01-worktree-worker-acceptance.md).
Rust: 41 passed, and the complete shell/Python suite passed (212 Python cases, one existing
optional Resurrect skip), as did both release-plugin assembled workflows.
All 22 user stories passed in the executed Linux / tmux 3.4 environment. Real terminal
matrices, 36-worker measurements and the corrected narrow-shell regression are
recorded in the report; final independent review follows in ticket15's receipt.

Implementation/Linux acceptance does not mean shipped: native macOS and ARM64
runtime, real third-party service sessions and manual release-artifact gates
remain unverified. These limitations have not been accepted as a shipping
decision. `main` remains `eaf24469290cbf77dd1d2a6176fbd54f7ace1868`; no push,
publication or deployment occurred. The historical checkpoints below are retained.

## Historical resumed checkpoint — 2026-10-01

The user resumed all remaining tickets on `work/worktree-worker-workflow`.
Ticket 12 starts at `a0075def1b75fc7e2f053d59cf0537f6b3353bc9`; tickets 13–15
follow their dependencies and independent reviews. The historical pause receipt
below records the previous stopping point and is superseded by this resumption.

Ticket12 implementation and its final frozen suite are complete. Independent
Standards/Spec review remains pending before ticket13 proceeds; see its receipt.

## Historical pause checkpoint — after ticket 11

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
