---
status: accepted
date: 2026-10-04
proposed-by: Memnoc
approved-by: Memnoc
---

# ADR-0008: Preserve the worktree worker workflow and its approved interface

## Context

A three-worker website session exposed fragile task delivery, confusing window
identity, coupled terminal navigation and manual integration work. The branch
`work/worktree-worker-workflow` implements the resulting workflow and successive
live-use UI corrections; Memnoc requested this durable checkpoint before
returning the checkout to `main`.

## Decision

Retain the stateless Rust supervisor and explicit integration workflow described
in [ADR-0002](0002-rust-stateless-v2-architecture.md),
[ADR-0005](0005-project-sessions-and-explicit-integration.md) and
[ADR-0007](0007-global-cockpit-and-navigation-roles.md), with the final Balanced A
status design recorded in [ADR-0006](0006-two-row-status-and-persistent-attention.md).
Users launch named workers with separate task instructions, supervise them in
Cockpit, and integrate through Drudwyn while preserving coordinator identity
and independent terminal navigation. Preserve this implementation on its branch;
returning to `main` does not merge or ship the work.

## Considered options

- **Stateless supervision with explicit integration** — chosen to retain tmux
  and Git as the source of operational state while removing repetitive shell work.
- **Persistent orchestration registry** — deferred at Memnoc's request; session
  recovery must not pretend to restore an agent's conversation.
- **Always integrate directly into main, or always create an integration branch**
  — neither is universal; the user chooses the destination, and coordinator
  conflict handling, verification and promotion remain explicit steps.
- **Many workspace tabs in the ambient bar** — retained as optional Dense/Tabs
  layouts; the approved default shows one workspace with three information
  regions, leaving the full inventory to dedicated overviews.

## Preserved implementation

- Tickets 01–15 cover launch-failure preservation, independent clients, stable
  coordinator/window identity, new shell sessions, pinned worker batches,
  separate short names and complete tasks, evidence-backed lifecycle/exit state,
  recovery, global Cockpit, integration, coordinator conflict handoff,
  assembled verification, explicit promotion and guarded cleanup.
- Cockpit, Workspace and Sessions share Rosé Pine (not Moon), the hound masthead,
  readable tables, detail panels and bot/manual-shell icons. Workspace groups
  agents/workers above manual shells/editors with separate scrolling; filtering
  preserves the selected window when it still matches.
- **Balanced A** is the final default: current workspace/icon/activity left,
  selected branch and tracked Git +/- physically centered, global NEED and
  agent total right. One information row and one subtle separator occupy two
  rows total, respecting top/bottom placement. Branches shorten before agent
  totals disappear; below 40 columns compact Focus rendering applies.
- Activity, process exit, review, integration and assembled verification are
  distinct evidence. Git line counts are not progress percentages; opening a
  workspace does not clear attention. No persistent conversation storage is added.

## Verification and outstanding work

The last tested implementation commit is `5a2c1d1` (2026-10-01). Its
[ticket24 receipt](../../.scratch/worktree-worker-workflow/24-balanced-status.md)
records the full repository gates and final assembled rerun after updating old
Focus assertions: 43 Rust library tests, four Balanced regressions, 30 independent
navigation tests, 11 settings tests and both assembled integration routes passed.
One existing optional tmux-resurrect test was skipped. Final Standards and Spec
crosschecks had no remaining findings; these are historical results, not new
2026-10-04 executions.

The [assembled acceptance report](../specs/2026-10-01-worktree-worker-acceptance.md)
records Linux/tmux runtime evidence, controlled agents and 36-worker inspection.
Native macOS/ARM64 runtime, real third-party agent-service arrangements and
manual release-artifact gates remain unverified; branch completion is not a
shipping decision.

**Bug at the original checkpoint:** [ticket19](../../.scratch/worktree-worker-workflow/19-codex-hook-ownership.md)
reproduced Codex hook failure when launcher/server/helper processes create
ambiguous ownership. UI polish did not fix it. Resume by validating content-blind
process-family and client/pane attribution with fake process families and private
tmux fixtures, retaining ambiguity for genuinely independent agents.

## Recall and resume

- Implementation branch: `work/worktree-worker-workflow`.
- Last implementation: `5a2c1d1`; grouping: `4d1df4d`; approved navigators: `1130dd8`.
- Main at checkpoint: `eaf24469290cbf77dd1d2a6176fbd54f7ace1868`.
- [Specification](../specs/2026-09-30-worktree-worker-workflow.md),
  [ticket plan](../specs/2026-09-30-worktree-worker-ticket-plan.md),
  [ticket index](../../.scratch/worktree-worker-workflow/README.md).
- Final status proposal archive: `spike/three-region-status-20261001`, `6348923`.
  Navigator proposal archive: `spike/workspace-session-20261001`, `dd1130f`.
  Git refs are the durable artifacts; `/tmp` previews and logs may disappear.

Read this record from any branch without switching:

```sh
git show work/worktree-worker-workflow:docs/adr/0008-worktree-worker-workflow-branch-checkpoint.md
```

Resume implementation with a clean checkout:

```sh
git switch work/worktree-worker-workflow
```

The release binary and live tmux presentation were applied on 2026-10-01.
Switching Git branches alone does not rebuild that binary or reload tmux;
validate source/binary alignment before further live-runtime testing.

## October 4 live-feedback implementation

All feedback in [ticket25](../../.scratch/worktree-worker-workflow/25-live-worktree-test-findings.md)
is now implemented on the retained feature branch. Action screens use the same
Rosé Pine language, visible editing/focus and result states; destinations are
selectable branch/checkout pairs. Verification returns fresh evidence to the
result. Recovery retains validated live tmux context without a persistent registry.
Coordinator return uses prefix + G, preserving tmux's existing prefix + C.

Task delivery waits for editor terminal mode but still distinguishes transmission
from acceptance. Default managed Codex uses --no-daemon; configured hooks prove
terminal ancestry plus process cwd and reject shared-backend attribution. This
resolves the supported launch route while retaining an explicit limitation for
existing shared-daemon sessions and platforms with unobservable process cwd.

Runtime snapshots now contain matching binaries/scripts/assets and atomically
select a stable installation path. Source-branch switches cannot remove it;
loader migration replaces only Drudwyn's own hooks and observer. Existing active
agent conversations are not restarted as part of installation.

43 Rust tests, the repository's integration/UI/privacy/package phases, targeted
failure reruns, and both assembled release workflows pass; one optional Resurrect
test is skipped. Independent Standards and Spec reviews have no remaining findings.
The [comparison plan](../specs/2026-10-04-worktree-feedback-comparison.md) preserves
the same baseline/tasks and separates automated evidence from pending real-agent
acceptance and visual judgement. Main remains eaf2446, clean and unmerged.

## October 4 second feedback pass — guided workflow

The live repeat established hook-backed completion, merging and passing checks,
but still required manual Enter after task paste and explanations from the
coordinator. [Ticket26](../../.scratch/worktree-worker-workflow/26-guided-workflow-polish.md)
implements the resulting operational and presentation improvements.

The common flow now says Start a task → review → Merge changes → Run checks.
Task setup exposes main versus a separate merge branch. Wide screens have focused
help; F1 opens scrollable explanations at any width without losing input. F8
keeps setup hashes in Details. Stable source/project folder identity accompanies
merge choices and results. Already merged offers follow-up actions rather than
another merge confirmation. Check receipts use separate lines and relative times,
while worker-reported checks remain distinct from checks of the merged result.

Shared Rosé Pine frames retain the hound, bot/shell icons, clear focus, quiet
separators and task borders. Short terminals use a compact masthead to preserve
editing space. Agent open describes process presence; Ready to review does not
mean approved, merged or verified.

Linux transport now waits for the paste's unread tty byte count to drain before
one submit, with a bounded timeout and no automatic replay. Only metadata is
observed. The delayed-editor regression passes; the next real-agent comparison
must still establish whether this removes the manual Enter in Codex. Other
platforms retain the earlier timing behavior and are not claimed validated.

Second-pass checks passed across Rust, Git/tmux operations, keyboard and visual
regressions, privacy, packaging and both assembled release routes. Independent
reviews found no remaining blockers. One optional session-restore test was skipped.
The stable runtime snapshot can be updated without switching or modifying main.
See ticket26 for exact receipts and the remaining live acceptance distinction.
