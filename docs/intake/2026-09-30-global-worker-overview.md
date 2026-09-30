# Global overview for dozens of workers

Date: 2026-09-30. Reporter and decision owner: Memnoc.
Status: Memnoc approved the global overview preview and surface role split;
recorded in [ADR-0007](../adr/0007-global-cockpit-and-navigation-roles.md).
The preview uses simulated data. No production behavior has changed.

## Source and problem

After accepting status-bar variant A and reviewing its tab cap and padding,
Memnoc requested an overview for “dozens of agents” that translates the same
design into “a global overview of what's happening.” This extends the
[worker workflow](2026-09-30-worktree-worker-workflow.md) and
[status feedback design](2026-09-30-status-feedback.md).

A bounded status bar remains useful only if its hidden workers are easy to
find, inspect, and act on. The global surface must scale without squeezing more
tabs into the bar or implying that hidden workers have stopped needing attention.

## Existing seams and accepted surface roles

[Cockpit discovery](../../src/discovery.rs) already starts from tmux windows
across sessions, and the [Cockpit](../../src/cockpit.rs) already has a filter,
selectable list, attention total, and detail pane. Its normal discovery excludes
ordinary shells; it does not establish the complete proposed coordinator and
recoverable-worktree inventory. Grouped-session window links also require
deduplication when building a global view.

The accepted responsibilities are:

| Surface | Purpose |
| --- | --- |
| Status bar | Bounded local window context plus clearly scoped global attention counts |
| Workspace navigator, `prefix + w` | Fast selection of any window, including ordinary shells |
| Session navigator, `prefix + s` | Find, create, rename, and close sessions; project-level attention summaries can be added without duplicating a full worker table |
| Workspace Cockpit, `prefix + P` | Global worker inventory, attention triage, details, coordinator access, and lifecycle/integration actions |

Global means the projects and sessions on the connected tmux server. It does not
introduce remote aggregation, a background service, or persistent worker history.

## Reviewed interaction direction

- The default overview spans all projects, grouped by project/session. Each
  group identifies its coordinator, worker count, and attention count.
- A global summary counts unique workers, failures, input requests, review,
  working, process-only running, and unknown activity. Filters do not silently
  change these global totals; the list separately reports matching/total counts.
- Search across names, branches, projects, sessions, and agent kinds. Offer
  project scope and All / Needs attention / specific-state filters. An optional
  attention grouping collects the same states across projects.
- Use the bar's established labels, worktree identity, colors, and spacing.
  Worker activity, Git cleanliness, integration, and verification remain
  separate columns or detail fields.
- Keep the currently open workspace's red dot distinct from the list cursor.
  Selecting a row inspects it; an explicit Open action jumps to its live window.
  Merely inspecting or opening a worker does not clear its attention state.
- Use a scrollable inventory with keyboard navigation rather than a tab cap.
  Preserve selection by stable identity during refresh and filtering where the
  selected worker remains visible; make an empty result explicit.
- At smaller widths, hide optional columns and expose their values in details.
  Preserve names, worktree/agent identity, and state. Narrow detail presentation
  must not make the worker unreachable.
- Clicking a status-bar failure/input/review count opens that global filter.
  The +N overflow route opens the current project's inventory. The preview
  demonstrates these paths; final tmux bindings and click behavior belong in
  the implementation spec.
- Full details include source branch, chosen integration target, lifecycle
  evidence, and verification status. A Review event does not prove successful
  verification or integration readiness. A conflict routes to the coordinator.

## Preview

The existing throwaway branch `spike/status-feedback-20260930` now includes
`?view=overview` on the same route:

`http://127.0.0.1:8786/status-feedback.spike.html?variant=A&view=overview`

Run `python3 scripts/status-feedback.spike.py` from its worktree. The current
review checkout is `/tmp/drudwyn-status-feedback-spike`.

The shared fixture has 36 workers across four projects, plus coordinator and
shell windows, for 44 unique window identities. Status-bar navigation is scoped
to the selected project; attention totals are global and explicitly labelled
ALL. Fixture metadata is shared by both views. No backend mutation is wired up:
Open changes the simulated current workspace, and integration-related buttons
describe the intended action without executing it.

Browser inspection exercised global counts, project and state filters, search,
attention grouping, an empty result, selected-worker details, opening a worker,
and drilling from a status-bar badge into the corresponding global filter.
All 36 rows were reachable in the scrollable list. A 780-pixel-wide browser
preview was inspected with optional columns collapsed and details below the list.
These observations establish prototype behavior, not production discovery
performance or real tmux input/rendering behavior.

## Acceptance agenda for the implementation spec

1. A fixture of dozens of unique workers across several projects remains
   discoverable, filterable, and navigable without depending on bar tab limits.
2. Linked copies of a window across independent terminal views count once.
   Selecting/opening workers affects only the requesting client as agreed.
3. Global counts agree between the bar and Cockpit on the same snapshot.
   Counts for filtered and collapsed groups are unambiguous. A hidden urgent
   worker remains represented and reachable through its alert.
4. Refresh preserves selection by identity; a disappeared window produces an
   explicit unavailable/recovery state rather than targeting a different worker.
5. Roles, activity evidence, Git integration, and verification remain distinct;
   unknown metadata is displayed as unknown and does not become false progress.
6. Search, long or identical names, narrow terminals, redaction, and both icon
   modes retain enough identity to safely select the intended workspace.
7. Existing ordinary agents, coordinator shells, and recoverable worktrees fit
   their respective views without silently vanishing or being counted as live
   working agents. Measure refresh costs with many local Git checkouts before
   choosing refresh frequency; no performance result is claimed by this spike.

## Review outcome

Memnoc approved the preview on 2026-09-30: “Looks really good - and I agree
with the role split you have proposed”. ADR-0007 records that decision.
The [worker-workflow spec](../specs/2026-09-30-worktree-worker-workflow.md) now
captures the acceptance agenda alongside the status-bar design. The
[ticket breakdown](../specs/2026-09-30-worktree-worker-ticket-plan.md) is approved
and published as executable tickets on the separate implementation branch.
