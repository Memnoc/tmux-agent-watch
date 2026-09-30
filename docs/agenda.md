# Update agenda

## Worktree worker workflow — design in progress

The [2026-09-30 field-report intake](intake/2026-09-30-worktree-worker-workflow.md)
organizes Memnoc's worktree session feedback, implementation evidence, open
questions, and the Northstar route. The
[worker workflow specification](specs/2026-09-30-worktree-worker-workflow.md)
now captures the accepted design; its
[15-ticket breakdown](specs/2026-09-30-worktree-worker-ticket-plan.md) is approved.
The [executable queue](../.scratch/worktree-worker-workflow/README.md) is being
implemented on `work/worktree-worker-workflow`.

[ADR-0005](adr/0005-project-sessions-and-explicit-integration.md) records the
agreed direction: a clearly identified shared project session, a New Session
action for ad hoc shells, explicit integration after review, and stateless task
associations. The coordinator agent handles merge conflicts; enforced port
reservations are deferred to a follow-up. Starting-point and integration-target
selection stays explicit: ask whether to merge directly into the configured
base (normally `main`) or create an integration branch for the batch.

[ADR-0007](adr/0007-global-cockpit-and-navigation-roles.md) records the approved
overview and role split: status bar for awareness, Cockpit for global
supervision, and window/session navigators for movement and session management.

- [ ] Reproduce missing planning windows, early worker exits, and terminal mirroring.
- [ ] Make launch simple: separate short names from tasks and select an explicit starting branch.
- [ ] Offer “Merge into main” or “Create an integration branch” when setting up the work; use the configured base's actual name, show the chosen destination, and retain the choice for the live batch.
- [ ] Identify workers and the coordinator across navigation and status surfaces.
- [ ] Implement status-bar variant A with [two informative rows, persistent badges, and configurable visible-tab limits](adr/0006-two-row-status-and-persistent-attention.md). Preserve worktree identity, selected-window visibility, and attention counts for hidden workers; expose density in the options editor and tmux configuration.
- [ ] Extend the [global Cockpit overview](intake/2026-09-30-global-worker-overview.md) for dozens of workers: shared status vocabulary, project/attention grouping, search and filters, full worker details, coordinator access, and drill-through from bar counts.
- [ ] Add a New Session command/action in Drudwyn, accessible from the session navigator alongside rename and kill; choose a name and starting directory, create a shell session, and switch only the requesting terminal to it.
- [ ] Recover existing worktrees when their terminals disappear.
- [ ] Integrate worker commits from the cockpit, with coordinator-agent conflict resolution, Continue/Abort actions, and assembled verification.

### Follow-up after the core workflow

- [ ] Design enforced shared-port reservations and resource coordination (deferred by Memnoc on 2026-09-30).

## Next update

- [ ] Add a guided workflow to recover unfinished work from an old branch.
  - Identify commits still missing from the base branch, including after a squash merge.
  - Show the proposed commits and destination base for review.
  - Create a fresh worktree from the configured base and cherry-pick the selected commits.
  - Guide conflict resolution and hand off validation and PR creation to the workspace agent.
  - After integration, offer cleanup of the old worktree and branch, accounting for branches still checked out elsewhere.

Acceptance scenario: `chore/drudwyn-rename` contains five branding commits added
after the rename was squash-merged. Recover those commits onto current `main`
without replaying the merged rename or losing the later license and worktree-base
changes. Preserve the source branch until the recovered work is integrated and
cleanup is explicitly confirmed.
