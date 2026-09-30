# 09 — Supervise workers in a global Cockpit

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** done — independently reviewed through `12c05ce`; Standards and Spec clear of blockers

**What to build:** The approved global overview becomes a real terminal inventory
for dozens of workers, with consistent evidence and reliable inspection/opening.

**Blocked by:** 03, 07.
**Priority:** P1. **Stories:** 13, 14, 15, 16, 22.

- [x] Discover at least 36 unique workers across four projects; deduplicate
  linked session windows and retain unassociated ordinary agents.
- [x] Provide project/attention grouping, name/ref/session/project/agent search,
  project/state filters, global totals, matching counts, and explicit empty results.
- [x] Expose coordinator access without counting shells as agents; a project
  inventory route can also reach non-worker windows hidden by bar overflow.
- [x] Separate the current-window marker from row selection; inspection does
  not navigate or clear attention. Preserve selection by ID through refresh.
- [x] Show complete selected details, including unknown batch/integration/check
  fields until evidence exists; do not invent data or show placeholder actions as working.
- [x] Narrow layouts, long/identical names, redaction, icon modes, and keyboard
  scrolling retain safe selection and access to all details.
- [x] Capture representative refresh costs and reuse per-refresh metadata;
  failure/staleness is visible and cannot retarget a pending action.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.

## Implementation and verification receipt — 2026-09-30

- Added a shared in-memory inventory snapshot for `cockpit --list` and actual
  Cockpit project/state/search/group views. Discovery deduplicates linked window
  IDs, includes ordinary unassociated agents, and infers only project-session
  membership for unassociated windows. Project-window inventory includes shells
  outside Git; their Git association remains unknown. Coordinator roles remain
  reachable without contributing to worker totals. Exited historical workers
  remain visible separately from running-process counts.
- The disposable large fixture has 36 workers in four Git projects, 35 linked
  worktrees and four primary checkouts (39 unique Git checkouts), coordinator
  shells, an additional shell in `/tmp`, and a linked independent client view.
  A further unassociated agent remains reachable. CLI and actual terminal tests
  verify counts, project-window routing, all rows via scrolling, and independent
  client selections. No installed real agent was used: fixtures assert the
  disposable `codex` executable resolves to `sleep`.
- Added project/attention grouping; name, ref, session, project and agent search;
  project/state filters; global and matching counts; explicit empty results;
  current-window `*` separate from inspected-row `>`; and full details with
  scrollable identity, refs, source/destination, task reference, process/evidence,
  exit and changed-file names. Unknown integration/check evidence is not replaced
  with guessed receipts or nonfunctional actions. Existing New, Batch, Recover,
  Finish and Coordinator controls remain available.
- Selected window IDs survive refresh/filter/group changes. A disappeared target
  fails explicitly, and a pending Finish cannot target a same-name replacement.
  Refresh runs outside the input loop, visibly retains the old snapshot while
  REFRESHING, and blocks actions until success. Failure marks the retained data
  STALE and permits inspection while requiring a successful retry for actions.
  Inspection/opening preserves attention and the other attached client's view.
- Git identity/status/names are cached once per unique checkout per refresh;
  batch records are loaded once per live batch. No Git/process probes occur in
  rendering, filtering or individual cells. The prior window projection used
  roughly twenty tmux subprocesses per worker; projection now reads updated pane
  fields once and sends one mutation queue under the same lifecycle guard.
  Mutation children retain the lock descriptor after parent death. Existing
  activity, scan/hook contention and orphan-write regressions all passed.
- Nine global command/UI cases cover the large inventory, details and inspection,
  in-flight refresh responsiveness/failure, reorder/filter/disappearance,
  lifecycle filters and exits, widths/icons/redaction, retained batch/task refs
  and metadata loss, coordinator actions/themes, and pending Finish identity.
  Red cases established the absent CLI route, absent branch search, blocking
  refresh, and clipped final detail lines before correction. The final audit
  also caught and fixed coordinator routing for ordinary project-session agents.
- Actual terminal tests cover 48/64/80/120/160 columns, safe/Nerd icons, label
  redaction, all three themes, and long/identical names. Width/icon/redaction
  captures are listed below. Forty-five changed-file
  names plus the final details/actions are reachable. Ratatui's pinned rendered
  line-count feature measures the actual inner area for complete wrapping.
  The existing 100×24 batch-destination test caught a regression caused by
  secondary identity fields; those fields now follow the batch summary. The
  unchanged assertion passed before restarting the final gates. The existing
  real-client suite also caught coordinator shells omitted from name search;
  search now retains coordinators with explicit separate matching counts, while
  global worker/live totals exclude them. The original navigation assertion
  remains unchanged, with all real-client tests rerun before refreezing.

### Performance and terminal evidence

Measured on Linux 7.1.5, tmux 3.4, Git 2.43.0 and Cargo 1.95.0 using disposable
`/tmp` repositories and two attached 120×40 terminal clients. Early metadata-only
36-worker discovery took 2.669 seconds. After projection consolidation, repeated
full snapshots including the 39 checkout metadata sets took approximately
1.15–1.17 seconds; the actual 36-worker UI was ready in 1.10–1.15 seconds.
End-to-last-row inspection took about 29 ms. During a deliberately blocked
refresh, opening full details took 4–29 ms while the child was still paused.
These are local fixture observations, not a production latency guarantee.
Explicit refresh, shared metadata and visible snapshot age avoid a silent polling
cost or a claim that old data is current.

Saved actual terminal evidence:

- `/tmp/drudwyn-ticket09-global36-120.txt`
- `/tmp/drudwyn-ticket09-current-vs-selected.txt`
- `/tmp/drudwyn-ticket09-empty.txt`
- `/tmp/drudwyn-ticket09-stale.txt`
- `/tmp/drudwyn-ticket09-{safe,nerd}-{48,64,80,120,160}-redact{0,1}.txt`
- `/tmp/drudwyn-ticket09-details-{safe,nerd}-{width}-redact{0,1}.txt`
- `/tmp/drudwyn-ticket09-details-end-{safe,nerd}-{width}-redact{0,1}.txt`

Focused evidence includes `/tmp/drudwyn-ticket09-activity.log` (36 tests),
`/tmp/drudwyn-ticket09-recovery.log` (17), `/tmp/drudwyn-ticket09-launch.log` (15),
and the global fixture logs. Final full-gate evidence is recorded below after
completion.

Limits: global scope is one connected tmux server. Metadata is an explicitly aged
snapshot; unavailable Git/batch fields stay unknown. Unmanaged literal paths
use Linux native cwd metadata when available, with directly resolvable tmux
metadata as fallback; no global unescaping or non-Linux runtime claim is made.
Integration, assembled verification and status-bar redesign remain later tickets.
Header singular/plural grammar is recorded for ticket15 polish (for example,
`1 projects`); it does not change counts or navigation.

Builder self-review checked the Standards and Spec axes sequentially, including
content-blind data flow, lock ordering/child lifetime, literal paths, role counts,
selection and pending actions, failed refreshes, all detail access and existing
control compatibility. Independent review remains pending; this receipt does not
self-clear it. `main` remains `eaf24469290cbf77dd1d2a6176fbd54f7ace1868`.

Ticket10 integration seam: `inventory::Totals::from_workspaces(&[Workspace])`
and `inventory::is_worker` perform no I/O. The status bar can consume the
existing deduplicated `discovery::discover_tmux()` lifecycle/project metadata
(which does not reconcile Git), or reuse an existing snapshot. It must not call
full `inventory::Snapshot::capture()` on every redraw merely to count workers
or attention. Cockpit's optional Git/batch detail enrichment is separate from
these shared counting rules; this ticket implements no status-bar redesign.


### Final frozen-code gates

`cargo fmt --check`, `git diff --check`, `cargo test --locked` (41 tests), and
complete `bash tests/run.sh` passed. The final run included 15 launch, 17 recovery,
9 global Cockpit, 36 activity/concurrency, 29 real-client, 13 navigator (one
existing optional Resurrect skip), and 10 settings tests, plus all shell,
lifecycle, privacy, packaging and release checks. Full log:
`/tmp/drudwyn-ticket09-final-suite.log`. No implementation changes followed this
successful frozen run. Earlier gate failures were the preserved destination
layout and coordinator name-search regressions described above; both original
assertions passed in the final run.

The final 36-worker/four-project/39-checkout fixture measured 1152 ms snapshot,
1.165 s CLI wall time, 1.119 s actual UI startup, 30 ms End-row inspection, and
5 ms inspection during a deliberately blocked refresh. Counts and environment
are those described above; measurements are fixture-specific.

Implementation and this verified ticket receipt are committed together on
`work/worktree-worker-workflow`. Independent review remains pending.

### Independent-review corrections (pending re-review)

The first independent review found two P2 defects, both reproduced unchanged by
`/tmp/drudwyn-review09-counting.py`; red output is retained in
`/tmp/drudwyn-ticket09-counting-red.log`.

1. Pane cwd had been used as the Git detail-cache key. Root and nested workers
   in one worktree reported two checkouts and ran two status probes. The cache
   now resolves the canonical checkout root, common directory and per-worktree
   Git directory before sharing branch/commit/status details. Identity lookup
   is shared per canonical cwd; detail lookup is shared per actual checkout.
   Linked branches retain separate identities. Selected details and navigation
   keep the authoritative pane cwd. Failed identity resolution remains unknown
   and does not skip retained batch/task metadata. Out-of-Git paths are not
   counted as identified checkouts.
2. Failed counted only the retained lifecycle label. Review plus a known native
   exit23 displayed a failed exit but disappeared from Failed filtering/counts.
   Shared pure predicates now include known nonzero/signal exits without
   rewriting hook history. Failed, Input and Review may overlap; the CLI and UI
   label overlapping categories. Worker and attention totals count each worker
   once. Failed grouping takes precedence while row/details retain Review and
   the native exit evidence. Zero/unknown exit alone is not failure or task
   completion. `Totals::from_workspaces`, state filtering, attention grouping
   and search consume these predicates; ticket10's no-I/O count seam therefore
   inherits the rule without Git scans.

The checkout regression failed at six reported checkouts instead of three, then
passed with exactly one status probe per actual checkout. It covers root,
nested, symlink and literal-dollar/semicolon/space paths, two distinct linked
branches, actual detail access and requester-only stable-ID navigation. The
failure regression failed at failed0/MATCHING0, then passed for Failed, Review,
Attention and Exited filters; it checks unique totals, retained hook timestamp,
native exit23, native signal, replacement invalidation and zero exit. Actual
48/80/120-column Cockpit captures preserve the Failed group, Review label,
overlap explanation and snapshot status. Native tmux dead-pane visibility can
precede or lose the exit receipt when terminal EOF wins the reaping race; the
fixture briefly keeps its terminal open through reaping and explicitly waits
for the signal receipt. Production continues to retain unknown receipts.

Original unchanged reproof: `/tmp/drudwyn-ticket09-counting-green.log` now shows
one checkout/one status probe, and one Failed match with Review plus exit23.
Focused tests: all 11 global Cockpit cases and all 36 activity/concurrency cases
passed (`/tmp/drudwyn-ticket09-corrections-global.log` and
`/tmp/drudwyn-ticket09-corrections-activity.log`). Actual captures:
`/tmp/drudwyn-ticket09-failed-review-{48,80,120}.txt`,
`/tmp/drudwyn-ticket09-failed-review-group.txt`, and
`/tmp/drudwyn-ticket09-failed-review-details.txt`.

The corrected 36-worker/four-project/39-checkout fixture measured 1150ms
snapshot, 1.165s CLI wall time, 1.107s UI ready, 28ms End inspection and 26ms
inspection during a blocked refresh, in the same Linux/tmux3.4 environment
recorded above. These are fixture measurements, not portable guarantees.
Audit confirmed no lifecycle mutation/locking, launch, recovery, attention
clearing, privacy-boundary or navigation changes; only cache identity and
supervision classification changed. The authoritative-cwd fallback and
non-Linux runtime limitations above remain. Code is frozen for final gates;
independent re-review is pending.

Correction final gates passed without code changes after freeze:
`cargo fmt --check`, `git diff --check`, `cargo test --locked` (41 tests), and
complete `bash tests/run.sh` (15 launch, 17 recovery, 11 global Cockpit,
36 activity, 29 real-client, 13 navigator with the existing optional Resurrect
skip, 10 settings, and shell/privacy/package/release checks). Logs:
`/tmp/drudwyn-ticket09-corrections-rust.log` and
`/tmp/drudwyn-ticket09-corrections-final-suite.log`. The final 36-worker,
four-project, 39-checkout sample was 1147ms snapshot, 1.161s CLI, 1.110s UI
ready, 29ms End inspection and 5ms inspection during blocked refresh.
Implementation, regression tests, user guidance and this correction receipt
are committed atomically; independent re-review remains pending.

### Independent final crosscheck receipt — 2026-09-30

Reviewed the full ticket diff `98659f7...12c05ce` and correction diff
`f907b66...12c05ce` against this ticket, the approved specification and ticket
plan, CONTRIBUTING.md, CONTEXT.md, privacy documentation and ADRs 0005–0007.
Standards and Spec were reviewed sequentially under the implementation workflow.
The independent reviewer session was reused because of the harness thread
limit; it found the two original P2s and implemented neither the ticket nor
the corrections.

Standards: no blocking violation or new material smell found. The shared
snapshot reads fixed tmux/process/Git metadata without content inspection or
persistent storage. Typed checkout identity separates root/canonical identity
from pane cwd and distinguishes linked worktrees sharing a common directory.
Rendering and query operations reuse the snapshot. Lifecycle projection batching
preserves the existing guard and outstanding mutation-child ownership. Pure
counting/failure predicates introduce no Git or other I/O for future consumers.

Spec: no outstanding ticket-09 finding. The global inventory deduplicates linked
views, retains unassociated agents, and provides explicit worker/window scopes,
coordinator access, filters, groups and matching totals. Stable row identity,
current-window marking, non-navigating inspection, explicit stale/refreshing
states, revalidated actions and scrollable/redacted details remain intact.
The two P2s are corrected: root/subdirectory/symlink aliases share one actual
checkout's detail probes, while distinct linked branches remain separate;
known nonzero/signalled exits participate in Failed filtering, search, grouping
and totals without erasing the historical handoff. Attention and worker totals
count each worker once. Failed/Input/Review overlap is explicitly labelled,
and an overlapping row appears once in the Failed attention group. Zero or
unknown exit alone does not manufacture failure or completion. Tickets 10 and
later were not treated as missing scope; minor header grammar remains the
already-recorded nonblocking polish item.

Independent validation on frozen `12c05ce`: all 11 global command/UI cases
passed in 29.245 seconds; log `/tmp/drudwyn-review09-final-global.log`. Coverage
includes the 36-worker/four-project inventory, linked views, shell routing,
selection/refresh failures, narrow redacted details, coordinator controls,
checkout identity and preserved native failure handoffs. The original unchanged
`/tmp/drudwyn-review09-counting.py` was rerun; its output is retained in
`/tmp/drudwyn-review09-counting-final.log`. One root/nested checkout now reports
one unique checkout and one status probe. Review plus native exit23 now reports
one Failed match, one unique attention worker, retained Review and an explicit
overlap label. All fixtures used disposable Git/tmux resources and asserted fake
agents; no installed agent or live user server was used.

The builder's fmt receipt and frozen Rust/full-suite logs were inspected,
including 41 Rust tests, 36 activity cases and the existing optional Resurrect
skip; those broader gates were not rerun or represented as reviewer-owned
full-suite evidence. `git diff --check` passed; the tree was clean before this
receipt and `main` remains
`eaf24469290cbf77dd1d2a6176fbd54f7ace1868`. Runtime evidence remains Linux/tmux
3.4; non-Linux fallback behavior was not exercised. This commit changes only
the ticket status and independent review receipt.
