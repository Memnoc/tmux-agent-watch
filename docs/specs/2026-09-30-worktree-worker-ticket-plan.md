# Worktree worker implementation breakdown

Date: 2026-09-30. Status: approved by Memnoc; executable tickets published.
Spec: [Worktree worker workflow](2026-09-30-worktree-worker-workflow.md).

Memnoc approved this breakdown on 2026-09-30 and requested implementation on
a separate branch. The [executable ticket queue](../../.scratch/worktree-worker-workflow/README.md)
contains one file per numbered section with the spec, blockers, acceptance
criteria, and status. Implementation uses `work/worktree-worker-workflow`;
`main` remains unchanged.

## Proposed frontier and order

| # | Ticket | Blocked by | What it delivers |
| --- | --- | --- | --- |
| 01 | Preserve work on launch failure | None | Failed launches remain understandable and cannot discard worker-written files or commits |
| 02 | Navigate independently in two terminals | None | Every Drudwyn navigation action targets only its requesting client |
| 03 | Identify and return to the coordinator | 02 | Stable coordinator/worker/shell names and roles across navigation |
| 04 | Create a shell session from Drudwyn | 02 | A New Session command and navigator action with name/directory input |
| 05 | Choose the batch source and integration destination | 03 | Pinned sibling starting point and explicit direct-to-base/integration-branch setup |
| 06 | Launch a named worker with a complete task | 01, 05 | Short name, branch preview, comfortable task input, and truthful delivery results |
| 07 | Show evidence-backed activity and exit state | 01 | Process-only Running, hook-backed activity, durable live attention, and visible exits |
| 08 | Recover a worktree and deliberately restart its task | 03, 06, 07 | Reopen surviving work without duplicating a branch or inventing conversation recovery |
| 09 | Supervise workers in a global Cockpit | 03, 07 | Global inventory, grouping, filters, full details, and stable selection |
| 10 | Ship status-bar A with density controls | 09 | Readable two-row status, bounded tabs, global alerts, and overview links |
| 11 | Integrate reviewed worker commits | 05, 09 | Preview and execute fast-forward or clean divergent integration from Cockpit |
| 12 | Resolve integration conflicts through the coordinator | 11 | Visible agent handoff, Continue, Abort, and reconciliation after external completion |
| 13 | Verify the assembled checkout | 11 | Explicit checks tied to the target revision and distinguished from reported checks |
| 14 | Promote integration branches and clean up safely | 12, 13 | Explicit final merge to base and removal only of eligible worktrees |
| 15 | Harden the assembled workflow | 04, 08, 10, 14 | Complete two-client, three-worker workflow and terminal-scale acceptance evidence |

The initial frontier is 01 and 02. Numbers are dependency order, not a mandate
to serialize every ticket. In particular, 04 is independent of worker launch,
09 does not wait for integration, and 10 and 11 can follow 09 independently.
Shared source files may still require integration coordination between workers.
No speculative prefactoring ticket is needed: reuse the existing discovery,
workspace command, lifecycle, and UI seams. If implementation finds a necessary
broad refactor, stop and revise the breakdown before widening a ticket.

## Common working agreement

- Use a fresh implementation session per ticket. Read the spec and accepted
  ADRs; preserve existing working changes that predate this plan.
- Build the narrow complete behavior test-first where appropriate. Diagnose
  reported bugs with reproducible evidence; do not claim the historical cause
  merely because a related test now passes.
- Exercise the spec's command/UI seams with disposable repositories, an
  isolated tmux server, and controllable worker processes. Never use the user's
  real project branches or live server as automated fixtures.
- Keep roles, evidence provenance, task content, client targeting, and privacy
  consistent across each slice. Errors and cancellation are part of delivery.
- Update relevant help/configuration/use documentation with the behavior each
  ticket ships. Preserve legacy fallback and existing unrelated features.
- Commit code and ticket status together after the relevant checks, full suite,
  and Northstar crosscheck. Record actual results and remaining limitations;
  a browser preview does not establish terminal behavior.
- Integration of ticket branches belongs to the coordinating session. Report
  overlapping files and any shared test resource; enforced port locking is
  outside this release. No implicit push, deployment, or branch deletion.

## 01 — Preserve work on launch failure

**What to build:** A failed start returns a truthful error and an actionable
path to retained resources instead of removing work the worker may have created.

**Blocked by:** None — can start immediately.
**Priority:** P0. **Stories:** 6, 7, 21, 22.

- [ ] Reproduce immediate zero/nonzero exit, metadata attachment failure, and
  a worker that writes a file or commit before failing through the launch action.
- [ ] Distinguish resources never used by a process from resources it may have
  modified. Retain uncertain, dirty, or newly committed work without force deletion.
- [ ] Display the launch failure and retained worktree/branch identity; do not
  show success because creation alone or a process exit succeeded.
- [ ] Retrying cannot overwrite existing branches/paths or duplicate a surviving
  live worker. Correctable failures leave unrelated windows/checkouts untouched.
- [ ] Verify preservation through real Git state, including untracked files;
  adapt existing rollback expectations only where the spec intentionally changes them.

## 02 — Navigate independently in two terminals

**What to build:** Two clients can view the same project's windows and navigate
through Drudwyn without moving the other client's selection.

**Blocked by:** None — can start immediately.
**Priority:** P0 investigation/P1 delivery. **Stories:** 9, 15, 16, 21.

- [ ] Establish a reproduction using two actual attached clients and record
  which part is normal shared-session behavior versus a Drudwyn targeting defect.
- [ ] Exercise window/session navigators, existing Cockpit Open, and status
  navigation; target the requesting client and preserve the other selection.
- [ ] Provide independently navigable views of shared project windows without
  changing files/branches or duplicating worker processes.
- [ ] Resolve stable IDs and session membership; missing/ambiguous clients and
  vanished targets report errors instead of navigating an arbitrary client/window.
- [ ] Linked session views deduplicate worker identity and do not cause
  application-created views to be confused with unrelated user sessions.
- [ ] Repeated attach/switch/detach operations have a documented view lifecycle
  and do not destroy windows still used by another client.

## 03 — Identify and return to the coordinator

**What to build:** A project has a recognizable coordinator and named windows
with visible roles, with a reliable return action from worker navigation.

**Blocked by:** 02.
**Priority:** P1. **Stories:** 8, 15, 16, 22.

- [ ] Explicitly associate a coordinator workspace with the project context;
  support both a shell and an agent without counting the shell as an active worker.
- [ ] Distinguish linked-worktree worker, ordinary agent, coordinator, and shell
  in navigators and existing Cockpit details using text as well as optional icons.
- [ ] Names derive from a deliberate short name/branch fallback; process title
  changes do not replace managed names, and later user renames are preserved.
- [ ] Starting and switching among three workers preserves the coordinator
  window ID and checkout branch; a vanished coordinator offers explicit recovery.
- [ ] Same-name projects/windows, external unassociated agents, and label
  redaction remain navigable by stable identity.

## 04 — Create a shell session from Drudwyn

**What to build:** Users invoke New Session from a command or the session
navigator, enter a name/directory, and arrive in a shell.

**Blocked by:** 02.
**Priority:** P1. **Stories:** 9, 10, 21.

- [ ] Expose New Session alongside existing management actions and through the
  command interface; show its shortcut/help without colliding with save or kill.
- [ ] Collect name and directory with an invoking-workspace directory default;
  display the resulting named shell session in navigation.
- [ ] Switch only the requesting client and leave existing workers and the
  second client's selected window/session unchanged.
- [ ] Cancellation, duplicate/invalid names, invalid directories, missing client,
  and command errors preserve existing sessions and allow a corrected retry.
- [ ] Names/paths containing spaces or shell metacharacters are treated as data;
  creation does not implicitly create a worktree, change a branch, or start an agent.

## 05 — Choose the batch source and integration destination

**What to build:** Batch setup pins a worker source commit and asks once whether
to integrate directly into the configured base or a named integration branch.

**Blocked by:** 03.
**Priority:** P1. **Stories:** 2, 3, 4, 21, 22.

- [ ] Preview the source ref and resolved commit, including configured-base,
  current-branch, and explicit locally available ref choices; do not fetch implicitly.
- [ ] Retain the pinned source for sibling launches despite navigation, target
  merges, or later source-ref movement; changing it requires deliberate selection.
- [ ] Show the base's actual configured name, or collect an integration-branch
  name and starting point; keep the selected destination visible for the live batch.
- [ ] Locate an appropriate destination checkout or offer a dedicated one;
  preserve the coordinator's original branch and never reset an existing branch.
- [ ] Invalid refs, name/path collisions, dirty source expectations, cancellation,
  and metadata loss have explicit outcomes; source and destination are distinct.
- [ ] Multiple batches for one repository retain independent associations;
  metadata is live-only and a missing association is unknown rather than guessed.

## 06 — Launch a named worker with a complete task

**What to build:** A launch form accepts a short name, editable branch, source
preview, agent, and usable task input, then creates and instructs the worker.

**Blocked by:** 01, 05.
**Priority:** P1. **Stories:** 1, 2, 3, 5, 6, 21, 22.

- [ ] Short name and worker branch are independent of task prose; detailed
  instructions do not generate an oversized branch/directory name.
- [ ] Edit and paste multiline instructions at supported terminal widths, or
  select a repository task-file reference; show full content/reference for review.
- [ ] Validate task-file availability in the worker checkout, including the
  case where a file exists only as uncommitted planning work in the source checkout.
- [ ] Launch from the displayed pinned commit, in the intended project context,
  while preserving the coordinator and other terminal's selection.
- [ ] Report creation and task transmission separately; delayed startup, failed
  paste/submission, and uncertain delivery permit deliberate recovery without
  silently resending or claiming that the agent accepted/completed the task.
- [ ] Bind delivery to the intended pane/process, and remove transient buffers
  on success/failure. Retain only a deliberately selected file reference, never
  task text in options, arguments, logs, or files.

## 07 — Show evidence-backed activity and exit state

**What to build:** Users can distinguish a running process, reported task
activity, attention requests, and an agent that exited, across current surfaces.

**Blocked by:** 01.
**Priority:** P1. **Stories:** 6, 7, 13, 16, 22.

- [ ] A process without a supporting lifecycle event displays Running, not Working.
  Show evidence source and unknown state where attribution is unavailable.
- [ ] Supported events produce Starting/Working/Needs input/Review/Failed
  consistently; Review does not imply passing checks or integration readiness.
- [ ] Preserve inspectable exit code/time when available in live metadata;
  zero exit is distinct from task completion, and lost history remains unknown.
- [ ] Attention survives inspection and unrelated scans; newer authoritative
  evidence supersedes it, and a replacement process cannot inherit stale evidence.
- [ ] Split-pane/active-pane changes do not reclassify the wrong process or route
  events to a different worker; unknown multi-agent ownership remains explicit.
- [ ] Existing pending lifecycle work is reviewed and covered by behavior tests;
  fixed metadata replaces content inspection and no inferred percent/ETA is added.

## 08 — Recover a worktree and deliberately restart its task

**What to build:** Cockpit exposes surviving worktrees for known repositories
and allows reopening a shell or deliberately restarting an instructed worker.

**Blocked by:** 03, 06, 07.
**Priority:** P1. **Stories:** 7, 8, 11, 15, 22.

- [ ] Enumerate Git worktrees for the selected repository and distinguish those
  without live windows from active workers; include missing coordinator recovery.
- [ ] Reopen the existing branch/path without duplicating a worktree, resetting
  files, or overwriting a live worker; preserve dirty and untracked work.
- [ ] Present Open shell and Restart with task/reference distinctly, explaining
  that a fresh agent does not restore a conversation. Offer resume only when supported.
- [ ] Recover a live task reference where still available, or require explicit
  re-selection after metadata loss; never invent old prompts, checks, or exit history.
- [ ] Report deleted directories, locked/prunable worktrees, missing agents,
  unavailable task files, and racing window creation without destructive repair.

## 09 — Supervise workers in a global Cockpit

**What to build:** The approved global overview becomes a real terminal inventory
for dozens of workers, with consistent evidence and reliable inspection/opening.

**Blocked by:** 03, 07.
**Priority:** P1. **Stories:** 13, 14, 15, 16, 22.

- [ ] Discover at least 36 unique workers across four projects; deduplicate
  linked session windows and retain unassociated ordinary agents.
- [ ] Provide project/attention grouping, name/ref/session/project/agent search,
  project/state filters, global totals, matching counts, and explicit empty results.
- [ ] Expose coordinator access without counting shells as agents; a project
  inventory route can also reach non-worker windows hidden by bar overflow.
- [ ] Separate the current-window marker from row selection; inspection does
  not navigate or clear attention. Preserve selection by ID through refresh.
- [ ] Show complete selected details, including unknown batch/integration/check
  fields until evidence exists; do not invent data or show placeholder actions as working.
- [ ] Narrow layouts, long/identical names, redaction, icon modes, and keyboard
  scrolling retain safe selection and access to all details.
- [ ] Capture representative refresh costs and reuse per-refresh metadata;
  failure/staleness is visible and cannot retarget a pending action.

## 10 — Ship status-bar A with density controls

**What to build:** The accepted visual design renders in tmux with stable tabs,
selected context, persistent global attention, and functional Cockpit links.

**Blocked by:** 09.
**Priority:** P1. **Stories:** 12, 13, 15, 16, 22.

- [ ] Use both existing rows: local tabs above, selected context/global attention
  below; preserve the selected dot, distinct roles, and evidence-backed activity.
- [ ] Provide the reviewed default of four tabs and selected-only/three/six/Auto
  settings through the options editor and documented tmux configuration.
- [ ] Count every local window toward the cap, preserve selected visibility and
  stable order, and accurately report hidden windows without truncating padding away.
- [ ] Separate dot/index, name/badge, badge edges, and adjacent tabs; validate
  actual cell widths at 48, 64, 80, 120, and 160 columns and both icon modes.
- [ ] Global attention matches Cockpit's snapshot semantics regardless of tab
  cap; badge routes open matching global filters and overflow reaches all hidden
  project windows, including ordinary shells.
- [ ] Keyboard routes and mouse targets preserve client independence; redraw,
  resize, themes, malformed settings, and redaction do not create stale click targets.

## 11 — Integrate reviewed worker commits

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

## 12 — Resolve integration conflicts through the coordinator

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

## 13 — Verify the assembled checkout

**What to build:** Users launch explicitly selected checks in the integration
checkout and see truthful revision-specific results separate from worker reports.

**Blocked by:** 11.
**Priority:** P1. **Stories:** 19, 21, 22.

- [ ] Offer a visible verification action for user/coordinator-selected checks
  in the target checkout; do not guess commands or execute automatically on merge.
- [ ] Associate live results with check identity, checkout, tested revision,
  timestamps, and exit status, without retaining command output or task content.
- [ ] Distinguish reported worker checks, Not verified, Running, Passed, Failed,
  and stale/missing evidence; worker Review never manufactures a check result.
- [ ] Failed/interrupted checks leave integrated work intact and allow rerun;
  a revision/observed working-tree change invalidates the current-result claim.
- [ ] Include an untracked application input that makes assembled checks fail
  despite clean worker branches; show the tested environment's limits accurately.
- [ ] Losing tmux metadata resets unprovable verification status, while Git
  ancestry can still establish integration independently.

## 14 — Promote integration branches and clean up safely

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

## 15 — Harden the assembled workflow

**What to build:** A reproducible acceptance run proves the integrated features
work together, followed by a real terminal visual review and release evidence.

**Blocked by:** 04, 08, 10, 14 (transitively all preceding tickets).
**Priority:** Release gate. **Stories:** 1–22.

- [ ] Run the complete supported Rust and shell/Python checks on the assembled
  revision, including preexisting lifecycle/status changes and compatibility paths.
- [ ] Create a session, launch three named workers from one commit, preserve
  coordinator identity, independently navigate two clients, and recover an exited
  worker through Drudwyn actions using controlled agents.
- [ ] Integrate independent commits with one shared-file conflict, hand off to
  the coordinator, Continue/Abort as applicable, and verify the assembled checkout.
- [ ] Exercise direct-to-base and integration-branch workflows including explicit
  promotion, failed verification from an untracked input, rerun, and guarded cleanup.
- [ ] Inspect actual terminal status/Cockpit layouts at all specified widths,
  icon modes, themes, redaction, 36-worker scale, and independent client actions.
- [ ] Record discovery/rendering costs and responsiveness with many Git checkouts;
  investigate regressions rather than borrowing the browser spike's performance.
- [ ] Confirm documentation/help/configuration, content-blind data flow, no implicit
  push/deployment, and crosscheck findings. Record limitations before calling it shipped.

## Review outcome

Approved by Memnoc: “approved, let's implement on a separate branch so we don't
risk polluting main”. The ticket queue is published with all 15 tickets initially
ready and blockers enforced before starting each ticket.
