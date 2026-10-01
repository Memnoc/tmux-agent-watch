# Worktree workers: launch, supervise, recover, and integrate

Date: 2026-09-30. Owner: Memnoc.
Status: specification and implementation breakdown approved by Memnoc;
implementation is tracked on `work/worktree-worker-workflow`.

## Problem Statement

Working on several tasks should not require repeatedly finding the original
checkout, recreating terminal windows, pasting the same instructions, or running
merge commands by hand. The field report showed useful isolated work, but weak
visibility between creating a worktree, delivering a task, completing work, and
integrating it. Missing windows and early exits were observed; their historical
causes and any data loss were not established.

The status bar also becomes crowded before it can explain which windows are
worktree workers, what they are doing, and which need attention. Dozens of
workers require a complete overview beyond a limited set of tabs. Multiple
terminals must remain independently useful instead of following each other's
navigation.

## Solution

Users create a named worker from an explicit starting point, supply a task,
and supervise it from Cockpit. A recognizable coordinator remains available.
The status bar provides immediate awareness; navigators provide quick movement
and session management; Cockpit provides global supervision and actions.

Users can recover a surviving worktree, understand the evidence behind a
worker's state, and integrate reviewed commits without copying Git commands.
They choose direct integration into their base branch or a separate integration
branch. Conflicts go to the coordinator agent. Assembled verification and
cleanup are explicit later steps.

## User Stories

1. As a user, I want a short worker name independent of its task, so that a
   detailed instruction does not become an unreadable branch or window name.
2. As a user, I want to choose a source branch/ref and inspect its resolved
   commit, so that workers receive the intended instructions and shared base.
3. As a user, I want sibling workers to retain that starting commit even after
   I navigate or integrate another worker, so that accidental dependencies do
   not appear between independent tasks.
4. As a user, I want to choose direct-to-base integration or an integration
   branch once per live batch, so that both workflows are supported and the
   destination remains visible.
5. As a user, I want a comfortable multiline task editor or an explicit task-file
   reference, so that launch requires no follow-up task paste in normal use.
6. As a user, I want launch, delivery, process exit, and agent activity to have
   distinct evidence, so that a closed window or successful exit cannot be
   mistaken for completed work.
7. As a user, I want failed launches and recovery to preserve files and commits,
   so that partial work survives a terminal or agent failure.
8. As a user, I want a stable, visibly named coordinator and identifiable
   worktree/ordinary-agent/shell windows, so that I can return to the right place.
9. As a user with two terminals, I want navigation and session creation in one
   to leave the other's selection alone, so that both terminals remain useful.
10. As a user, I want a New Session command and session-navigator action with a
    name and directory, so that I can open a shell for editing or other work.
11. As a user, I want to reopen an existing worktree without creating a duplicate
    branch, and deliberately restart or resume its agent, so that recovery does
    not pretend to restore a lost task or conversation.
12. As a user, I want a two-row status bar with readable spacing and configurable
    tab density, so that selection, identity, and attention remain legible.
13. As a user, I want persistent attention counts for hidden workers and a route
    to their matching Cockpit view, so that a small bar does not hide urgent work.
14. As a user supervising dozens of agents, I want a global inventory with project
    and attention grouping, search, filters, and full selected-worker details,
    so that every worker is reachable without relying on tabs.
15. As a user, I want inspection to differ from opening a window, stable selection
    during refresh, and a clear unavailable state for a disappeared target,
    so that an update does not act on the wrong worker or lose my place.
16. As the supervisor, I need to deduplicate linked windows and preserve evidence
    provenance, so that counts and activity labels remain truthful across views.
17. As a user, I want to inspect source and destination commits and initiate
    integration from Cockpit, so that merging reviewed work is a built-in action.
18. As a coordinator, I want conflicts handed to my agent in the destination
    checkout with visible Continue/Abort actions, so that resolution remains
    deliberate and does not require the user to type Git plumbing.
19. As a user, I want reported worker checks separated from checks actually run
    on the assembled checkout, so that branch-level success is not presented as
    verified integration.
20. As a user, I want explicit promotion of an integration branch into the base
    and guarded cleanup, so that finishing workers does not silently ship or
    remove work.
21. As a user, I want understandable errors and cancellation for invalid names,
    collisions, dirty targets, stale refs, and unavailable tools, so that retrying
    does not reset an existing checkout or duplicate an action.
22. As a user, I want the existing content-blind and label-redaction behavior
    preserved, so that improved supervision does not create a conversation store.

## Implementation Decisions

### Accepted architecture and scope

Follow [ADR-0005](../adr/0005-project-sessions-and-explicit-integration.md),
[ADR-0006](../adr/0006-two-row-status-and-persistent-attention.md), and
[ADR-0007](../adr/0007-global-cockpit-and-navigation-roles.md).
Retain [the stateless architecture](../adr/0002-rust-stateless-v2-architecture.md)
and [tool-agnostic review](../adr/0003-keep-review-tool-agnostic.md).
The existing Rust implementation is the feature target. Legacy compatibility
bindings and native navigation remain usable; feature parity with the legacy
implementation is not required.

### Identity, discovery, and terminal views

Keep Workspace's existing meaning: a live tmux window. Represent a recoverable
linked worktree separately from its optional live workspace; it is not counted
as a running worker. Recovery enumerates Git worktrees for a selected known
repository, rather than crawling arbitrary filesystem locations.

A project is identified by its canonical Git common directory, not its display
name or currently checked-out branch. A live batch identifies its coordinator,
source ref and pinned commit, destination branch and checkout, and associated
worker windows. Multiple batches in the same repository must not borrow each
other's destination. External agents without batch metadata remain discoverable
with unknown associations, not guessed ones.

Deduplicate windows by tmux window ID across linked session views, retaining
their memberships for client-local navigation. Select a controllable agent pane
explicitly rather than assuming the active pane represents the whole window.
Ambiguous multi-agent windows must not receive a task or action silently.

All Drudwyn navigation actions resolve the invoking client and a stable target
identity. If the client or target is ambiguous or gone, report that condition;
do not fall back to moving every client or whichever window now occupies an
index. Independent views may use grouped tmux sessions, as demonstrated in the
earlier isolated experiment; their user-visible identity remains the shared
project, and linked views must not inflate worker counts. Exact view lifecycle
and naming are implementation choices verified with two actual attached clients.

Explicit user names take precedence over generated names. Generate window names
from the short worker name, with a branch-name fallback for recovered worktrees.
Prevent process-title updates from replacing a managed name with `zsh`, while
preserving deliberate user renames. Coordinator access works for a shell as
well as an agent, with recovery offered when its original window is gone.

### Setup, New Session, and worker launch

New Session is available as a Drudwyn command and from the session navigator.
Collect a name and starting directory, defaulting the directory from the invoking
workspace when available. Create a shell session without creating a worktree or
agent. Validate before creation; cancellation, duplicate names, missing clients,
and invalid directories produce explicit results without disturbing other sessions.

Batch setup resolves the chosen source to a commit using locally available refs.
Preserve the existing configured-base and explicit current-branch choices; allow
an explicit source selection and show what will be used. Do not fetch implicitly.
Pin the resolved commit for sibling launches. Uncommitted planning files are not
inherited by a new worktree; show that limitation when the source is dirty.

Show the integration destination using the configured base's actual name.
For a new integration branch, collect its name and show its starting commit.
Reuse a suitable existing checkout of the destination, or offer a dedicated
checkout if none exists; never change the original coordinator's branch to
accommodate a destination. Branch/path collisions require correction or explicit
selection of an existing target, never force-resetting. A destination change
must be explicitly reviewed before it affects later integration.

Launch collects short name, editable worker branch, source preview, agent, and
either multiline task text or a repository-owned task-file reference. The task
does not determine the branch slug. File references are resolved against the
chosen repository/worktree and checked for availability in the resulting worker
checkout. If a referenced file is not present there, explain that before task
delivery rather than launching an apparently instructed worker.

Task text travels only through the existing transient input channel. Retain a
task-file reference in live operational metadata when deliberately selected;
do not retain the task text, parse tickets into a registry, or read agent output.
The agent reads the referenced file. A successful paste/submission means sent,
not accepted or implemented; only supported non-content evidence can establish
more. Do not automatically resend an uncertain delivery and risk duplicate work.

Record launch stages and command failures as fixed operational metadata. Where
available, retain an exit code and time in the live workspace or batch context.
An exited agent should leave an inspectable/recoverable state. Startup cleanup
must not force-remove a worktree after a process may have written files or
committed; preserve uncertain or modified resources and offer recovery. Cleanup
of a provably unused allocation may remain automatic. An exit code of zero
does not mean the task passed its acceptance criteria.

Recovery distinguishes Open shell, Restart with task/reference, and any explicitly
supported agent resume path. It explains when a new conversation will start.
Loss of tmux metadata means reselecting the task and batch choices; Git still
establishes checkout existence, branch, and ancestry. Unknown historical exit
and verification state stay unknown.

### Activity and attention

Use the following presentation semantics consistently, including in compact
labels and summaries. Presentation labels may abbreviate without changing meaning.

| Activity | Evidence required |
| --- | --- |
| Starting | A launch attempt is in progress |
| Working | A supported lifecycle event reports activity |
| Running | An agent process exists without stronger activity evidence |
| Needs input | A supported lifecycle event requests interaction |
| Review | A supported handoff/turn-end event calls for inspection, without claiming task completion |
| Failed | An observed launch/command failure, nonzero exit, or supported failure event |
| Unknown | Evidence is missing or no longer attributable to the current process |

Show terminal/process exit separately, including a known zero exit, instead of
inventing a completion state. Associate lifecycle evidence with its pane/process
lifetime so a new process cannot inherit an old process's success or request.
Process scans must not erase valid attention merely because the process exists.
Opening/inspecting a worker never clears attention; new authoritative evidence
can supersede it. Preserve an unresolved handoff through a known exit while
showing that the process has exited. No percentages, ETAs, blinking, automatic
focus changes, or inferred work completion.

### Status bar and global Cockpit

Implement reviewed variant A as terminal-native UI, using the preview as a
visual reference rather than a runtime dependency. First row: stable local
window tabs. Second row: selected context and clearly labelled global attention.

Use four visible tabs by default, with selected-only, three, six, and Auto choices
matching the reviewed preview. Expose the preference through the options editor
and a documented tmux option. The cap includes shell/coordinator/worker windows,
excludes the overflow indicator, and may be reduced further by available width.
Always retain the selected window; preserve window order. Separate the selected
dot from the index, the name from its padded badge, and adjacent tabs. Prefer
fewer tabs before aggressive name truncation. Color supplements textual identity.

The workspace navigator remains the complete all-window route. Bar overflow
opens the project inventory with hidden shells/coordinators reachable as well
as workers; it must not advertise hidden windows that the destination omits.
Global failure/input/review badges open matching Cockpit filters. Every mouse
action has a keyboard route; exact new shortcuts must avoid existing bindings.

Cockpit starts with all known projects on the connected server. Provide project
grouping with coordinator access, attention grouping, project/state filters,
and search over worker/window names, branch, session, project, and agent kind.
Keep unassociated agents reachable. Global totals count unique workers and
remain unchanged by filters; matching counts and group counts have explicit scope.
Coordinator shells are accessible without being counted as running agents.

Separate current-window marking from row selection. Selection inspects details;
Open navigates. Preserve selection by identity during refresh and filtering when
still visible. Otherwise choose a deterministic visible row without activating
it; mutations always revalidate the selected identity. Show empty results and
unavailable/recoverable targets clearly. A scrollable inventory replaces any
tab limit. At narrow widths, move optional fields into details while retaining
enough name, role, and activity to select safely.

Details include full identity, source commit, task reference if known, destination,
activity evidence, Git state, changed-file names/counts, integration state, and
verification status. Do not read diffs, task contents, commit bodies, or terminal
content. Honor label redaction for all names, paths, refs, and task references.
Display missing fields as unknown, never manufactured receipts.

Build both projections from the same discovery/counting semantics. For the same
snapshot their totals must agree. Reuse metadata across a refresh; do not run
Git probes per status cell or treat stale results as current. Measure dozens-of-
workers discovery and rendering before choosing refresh tuning.

### Integration, conflict handling, verification, and cleanup

Integrate is available through Cockpit and the command interface. Preview the
worker branch and commit, target branch/checkout and commit, changed-file metadata,
and known checks. Revalidate source and target before mutation; changed commits
require refreshed review. Reject dirty source/target checkouts, untracked overwrite
risks, detached/ambiguous destinations, and an existing Git operation. Do not
stash, reset, or discard changes automatically. Serialize Drudwyn integration
operations per destination and respect Git's own locks.

Use a fast-forward when possible and a normal merge for divergent clean history;
an already-contained source is an explicit no-op. Do not squash or rebase silently.
Resolve the actual destination checkout rather than assuming the original shell
is on the destination. Report Git failures without inferring success from a
closed popup. Branch integration is established by current ancestry against the
selected target, not solely by a cached completion label.

On conflict, retain the merge in the destination and show a coordinator handoff.
Send that agent an explicit instruction with the relevant checkout and refs using
the transient task-delivery channel; do not parse or solve conflict contents in
the supervisor. If the coordinator is absent or its delivery is uncertain, offer
Open/recover/retry without silently starting another merge. Continue requires all
unmerged entries resolved and the expected operation still present; Abort uses
Git's normal abort behavior and reports failures without a destructive fallback.
If the coordinator independently completed or aborted the merge, reconcile
against Git rather than blindly repeating Continue. Conflicts never trigger
automatic cleanup, promotion, or verification success.

Provide an explicit verification action in the assembled destination checkout.
The user or coordinator selects the checks; Drudwyn does not guess project test
commands. Run them visibly and retain only a live non-content result: check
identity, checkout, tested revision, start/end state, and exit status. Agent-
reported checks remain labelled reported. Missing evidence is Not verified.
Verification failure leaves the merged work inspectable and rerunnable. Any
observed revision or working-tree change invalidates the current-result claim;
a clean Git status alone cannot prove an unchanged ignored/untracked environment.
Show when/where checks ran rather than claiming deployment or eternal validity.

Promotion of an integration branch to the base is a separate explicit integration
using the same preview, checkout, conflict, and verification rules. Verification
of one destination does not automatically certify a new merge in another.
Finish remains distinct: verify the worktree is clean, its commits are contained
in the explicitly selected destination, and no Git operation or active writer
will be interrupted. Confirm removal, preserve the branch, and close only windows
belonging to the removed worktree. Never use mere containment in an unrelated
primary-checkout HEAD as proof that the chosen destination received the work.

## Testing Decisions

The primary seam is Drudwyn's user-facing command/action interface. Exercise it
against disposable real Git repositories and an isolated tmux server with
controllable fake agent processes. Assert branches, ancestry, file preservation,
window/pane identities, client selections, metadata, results, and rendered output.
Fake only worker lifecycle/input timing and injected command failures; do not
replace Git merge or tmux client behavior with an in-memory imitation.

The secondary seam is the existing rendered UI/input interface: terminal buffers
for layout and actual key events for routing. Use controlled metadata snapshots
for deterministic visual/state cases, then real tmux interaction to prove actions.
Prior art includes the repository's navigator/settings interaction tests,
worktree and v2 integration tests, lifecycle fixtures, status-bar rendering tests,
and privacy checks. Extend these mechanisms instead of introducing a broad new
mocking framework or asserting private helper call sequences.

Required cases:

- Early zero/nonzero exit, slow startup, task submission failure, a process that
  writes/commits before failure, unknown delivery, and reopening after metadata loss.
- Three workers from one pinned commit, source-ref movement after preview,
  duplicate names/paths, missing task files, user rename precedence, and cancellation.
- Two attached clients navigating via both navigators, Cockpit, and bar links;
  New Session switches only its requester. Deduplicate linked windows and handle
  a vanished target without selecting an unrelated replacement.
- Hook-backed Working versus process-only Running; attention survives inspection
  and unrelated scans; process replacement cannot inherit stale lifecycle evidence.
- At least 36 workers across four projects, duplicate display names, all filters,
  empty results, off-screen attention, selected-row preservation, and matching
  global totals. Test widths 48, 64, 80, 120, and 160, both icon modes, supported
  themes, and redaction. Review the real terminal spacing, not just the browser.
- Fast-forward, divergent clean merge, already-contained source, controlled
  conflict, Continue, Abort, missing coordinator, stale refs, dirty targets,
  competing operations, and a target already checked out elsewhere.
- Both destination workflows, explicit promotion, failed and successful assembled
  checks, a changed revision, and an untracked build input that affects verification.
  Cleanup must preserve dirty/unmerged work and retained branches.
- No task text or hook payload in arguments, logs, files, or tmux metadata;
  no new terminal-content inspection, prompt history, or persistent worker registry.

Each implementation ticket supplies a failing behavioral regression where
appropriate, focused passing checks, and a Northstar crosscheck receipt. The
assembled pass runs Rust checks and the repository's complete shell/Python suite,
then the end-to-end scenario with three workers, two clients, recovery, one merge
conflict, verification, and cleanup through Drudwyn. Record performance measurements
for global refresh; the UI spike is not evidence of production performance.

## Out of Scope

- Enforced port/resource reservations — explicitly deferred to a follow-up.
- Shared-file conflict prediction — conflicts are handled during explicit integration.
- Durable task/session history or cross-server aggregation — outside the accepted
  stateless, local supervision design.
- Automated conflict solving by Drudwyn — the coordinator agent owns resolution.
- Automatic merging on completion, pushing, deployment, or branch deletion — these
  are not implied by worker completion or integration.
- Conversation recovery without an agent-supported resume path — a reopened
  terminal cannot reconstruct a lost conversation.
- Percent-complete, ETA, or productivity scoring — available evidence does not
  establish those facts.
- Recovery of post-squash commits from an old branch — remains a separate existing
  agenda item; this work supplies reusable navigation/integration behavior only.

## Further Notes

Sources: [field-report intake](../intake/2026-09-30-worktree-worker-workflow.md),
[status feedback](../intake/2026-09-30-status-feedback.md),
[global overview](../intake/2026-09-30-global-worker-overview.md), and the accepted
ADRs above. The throwaway preview is on branch `spike/status-feedback-20260930`,
overview revision `5104ad4`; it is a visual reference with simulated data.

The proposed test seams were presented to Memnoc during specification work;
any additional real-project trial can supplement the isolated checks. Exact
shortcuts, option spelling, grouped-view lifetime, and the verification-command
entry affordance are routine implementation details to resolve against existing
interfaces. No specific project test command or production refresh budget has
been supplied. Historical launch/window incidents still require reproducible
evidence before a ticket claims their cause has been fixed.

Existing lifecycle/status changes in the working checkout predate this spec.
Review and preserve them; do not count their unverified behavior as completed
tickets. [The implementation breakdown](2026-09-30-worktree-worker-ticket-plan.md)
maps this scope into reviewable vertical slices.

## Verification

2026-10-01: assembled acceptance executed on Linux x86_64 / tmux 3.4, on
`work/worktree-worker-workflow`. All 22 stories below passed in that environment.
The [acceptance report](2026-10-01-worktree-worker-acceptance.md) records the
reproducible release-plugin scenarios, complete suite, terminal artifacts,
performance measurements, corrected failure, and release limitations.

Final checks: 41 Rust tests passed; complete shell/Python suite passed (212 Python
cases, one existing optional Resurrect skip); release build and both assembled
flows passed. The two-client / three-worker flows cover New Session, stable
coordinator, deliberate recovery, real shared-file conflict/handoff/Abort/Continue,
untracked-input check failure/rerun, promotion and guarded branch-preserving
cleanup. Actual terminal checks cover five widths, both icon modes, three themes,
redaction, and 36 workers across four projects/39 checkouts.

| # | User story (verbatim) | Verdict | Execution evidence |
| --- | --- | --- | --- |
| 1 | As a user, I want a short worker name independent of its task, so that a detailed instruction does not become an unreadable branch or window name. | pass (Linux) | Worker launch form/task tests; assembled alpha/beta/gamma names and work/* branches. |
| 2 | As a user, I want to choose a source branch/ref and inspect its resolved commit, so that workers receive the intended instructions and shared base. | pass (Linux) | Batch source/ref previews, expected-commit confirmation and stale-source refusal; assembled pinned SHA. |
| 3 | As a user, I want sibling workers to retain that starting commit even after I navigate or integrate another worker, so that accidental dependencies do not appear between independent tasks. | pass (Linux) | Batch tests move source refs between launches; assembled three sibling HEADs equal the pinned SHA. |
| 4 | As a user, I want to choose direct-to-base integration or an integration branch once per live batch, so that both workflows are supported and the destination remains visible. | pass (Linux) | Both assembled runs; batch setup/reuse/collision and destination visibility tests. |
| 5 | As a user, I want a comfortable multiline task editor or an explicit task-file reference, so that launch requires no follow-up task paste in normal use. | pass (Linux) | Multiline launch UI and raw receiver tests; assembled multiline stdin and task-file launch/recovery. |
| 6 | As a user, I want launch, delivery, process exit, and agent activity to have distinct evidence, so that a closed window or successful exit cannot be mistaken for completed work. | pass (Linux) | Launch15, activity39 and status exit/attention cases; assembled retained exited beta and fresh receiver. |
| 7 | As a user, I want failed launches and recovery to preserve files and commits, so that partial work survives a terminal or agent failure. | pass (Linux) | Startup failure and recovery preservation checks; assembled beta commit survives stop/restart. |
| 8 | As a user, I want a stable, visibly named coordinator and identifiable worktree/ordinary-agent/shell windows, so that I can return to the right place. | pass (Linux) | Coordinator identity/navigation tests; assembled same coordinator window survives launch, conflict and cleanup. |
| 9 | As a user with two terminals, I want navigation and session creation in one to leave the other's selection alone, so that both terminals remain useful. | pass (Linux) | Two actual PTY-attached clients in navigation/status and both assembled runs; requester-only New Session and Cockpit Open. |
| 10 | As a user, I want a New Session command and session-navigator action with a name and directory, so that I can open a shell for editing or other work. | pass (Linux) | Session command and navigator keyboard tests cover directory/default, cancellation and invalid inputs; assembled New Session. |
| 11 | As a user, I want to reopen an existing worktree without creating a duplicate branch, and deliberately restart or resume its agent, so that recovery does not pretend to restore a lost task or conversation. | pass (Linux) | Recovery tests cover shell/restart/reference/resume availability and metadata loss; assembled deliberate task-file restart. |
| 12 | As a user, I want a two-row status bar with readable spacing and configurable tab density, so that selection, identity, and attention remain legible. | pass (Linux) | Installed two-row status decoded from real client ANSI at 48/64/80/120/160, 2 icon modes, 3 themes; cap/settings tests. |
| 13 | As a user, I want persistent attention counts for hidden workers and a route to their matching Cockpit view, so that a small bar does not hide urgent work. | pass (Linux) | Hidden attention, overlap, mouse and keyboard routes, off-screen workers and membership-race tests. |
| 14 | As a user supervising dozens of agents, I want a global inventory with project and attention grouping, search, filters, and full selected-worker details, so that every worker is reachable without relying on tabs. | pass (Linux) | 36 workers / 4 projects / 39 checkouts; global filters/search/grouping/details and full terminal matrix. |
| 15 | As a user, I want inspection to differ from opening a window, stable selection during refresh, and a clear unavailable state for a disappeared target, so that an update does not act on the wrong worker or lose my place. | pass (Linux) | Cockpit inspect/Open, selected-ID preservation and vanished-target tests; assembled inspect leaves both clients unchanged. |
| 16 | As the supervisor, I need to deduplicate linked windows and preserve evidence provenance, so that counts and activity labels remain truthful across views. | pass (Linux) | Linked-client inventory deduplication and lifecycle process-birth/provenance tests; global/status counts reconcile. |
| 17 | As a user, I want to inspect source and destination commits and initiate integration from Cockpit, so that merging reviewed work is a built-in action. | pass (Linux) | Integration20 includes actual Cockpit preview/cancel/apply and stale IDs/refs; assembled reviewed integration. |
| 18 | As a coordinator, I want conflicts handed to my agent in the destination checkout with visible Continue/Abort actions, so that resolution remains deliberate and does not require the user to type Git plumbing. | pass (Linux) | Conflict17 covers handoff delivery/retry/recovery, real UI Continue/Abort and replaced operations; both assembled conflicts. |
| 19 | As a user, I want reported worker checks separated from checks actually run on the assembled checkout, so that branch-level success is not presented as verified integration. | pass (Linux) | Verification10 covers actual visible terminal execution, failure/staleness/missing receipts; assembled untracked-input failure/rerun. |
| 20 | As a user, I want explicit promotion of an integration branch into the base and guarded cleanup, so that finishing workers does not silently ship or remove work. | pass (Linux) | Promotion/cleanup15, real Cockpit P/f, lock/writer/concurrency safeguards; assembled explicit promotion and branch-preserving cleanup. |
| 21 | As a user, I want understandable errors and cancellation for invalid names, collisions, dirty targets, stale refs, and unavailable tools, so that retrying does not reset an existing checkout or duplicate an action. | pass (Linux) | All command/UI suites include cancellation, collisions, dirty/stale/ambiguous/missing resources and bounded retry failures. |
| 22 | As a user, I want the existing content-blind and label-redaction behavior preserved, so that improved supervision does not create a conversation store. | pass (Linux) | Privacy and lifecycle suites, transient delivery checks, metadata-loss cases, full redaction matrix; no production content inspection added. |

Native macOS (x86_64/ARM64), Linux ARM64, real third-party service sessions,
manual native release-artifact installation/publication, and the unavailable
optional Resurrect integration are **unverifiable** in this environment. Linux
fixture execution does not establish those results. Memnoc has not accepted
these missing observations as a shipping decision; implementation and Linux
acceptance are complete, but this is not a shipped claim. The report explains
the process-observation and point-in-time verification limits.

A narrow shell-context regression introduced during the approved cosmetic
polish was observed, routed to a fresh fix session, and re-executed after its
correction. The report retains both failing and passing evidence. Final
independent review is recorded with ticket15. The project repository's `main`
remains `eaf24469290cbf77dd1d2a6176fbd54f7ace1868`; nothing was pushed or deployed.

## Visual follow-up — 2026-10-01

Memnoc approved the project-table Cockpit A from
`spike/cockpit-layout-20261001` at `df0d33b`, and requested the same masthead,
palette, separators, spacing and selection language in Workspace and Sessions.
[Ticket 20](../../.scratch/worktree-worker-workflow/20-approved-cockpit-and-one-row-proposals.md)
records the implementation and terminal validation. The browser reference is
archived on that throwaway branch, rather than merged into production.

Memnoc also requested single-row alternatives to the current status bar. The
three runnable designs live on `spike/single-row-status-20261001` at `316f73d`;
run `python3 scripts/status-feedback.spike.py` in its worktree and open
http://127.0.0.1:8788/status-feedback.spike.html?view=bar&variant=A.
A uses quiet tabs, B focuses on the selected workspace and Git, and C adds
project activity counts. Git changes and global attention remain visible at
48/64/80/120/160 columns. No variant has been selected; the two-row production
bar and ADR-0006 remain unchanged pending that choice.
