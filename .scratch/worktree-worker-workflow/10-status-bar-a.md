# 10 — Ship status-bar A with density controls

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** done — independently reviewed through `3c4f71f`; Standards and Spec clear of blockers

**What to build:** The accepted visual design renders in tmux with stable tabs,
selected context, persistent global attention, and functional Cockpit links.

**Blocked by:** 09.
**Priority:** P1. **Stories:** 12, 13, 15, 16, 22.

- [x] Use both existing rows: local tabs above, selected context/global attention
  below; preserve the selected dot, distinct roles, and evidence-backed activity.
- [x] Provide the reviewed default of four tabs and selected-only/three/six/Auto
  settings through the options editor and documented tmux configuration.
- [x] Count every local window toward the cap, preserve selected visibility and
  stable order, and accurately report hidden windows without truncating padding away.
- [x] Separate dot/index, name/badge, badge edges, and adjacent tabs; validate
  actual cell widths at 48, 64, 80, 120, and 160 columns and both icon modes.
- [x] Global attention matches Cockpit's snapshot semantics regardless of tab
  cap; badge routes open matching global filters and overflow reaches all hidden
  project windows, including ordinary shells.
- [x] Keyboard routes and mouse targets preserve client independence; redraw,
  resize, themes, malformed settings, and redaction do not create stale click targets.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.

## Builder receipt — pending independent review

Implemented against `2220f3f1fe2397ef4b3bf3e219daac782655aeb1` on
`work/worktree-worker-workflow`. The approved A screenshot and isolated spike
were inspected as visual references only. The new Rust module owns bounded
rendering and stable actions; explicit legacy mode keeps its clustered bar and
separator. Both existing Rust status rows now carry information, independent
of top/bottom status position.

### Verified behavior and seams

- Five density choices (`1`, `3`, default `4`, `6`, `auto`) are available in
  Appearance and `@drudwyn-visible-tabs`. Malformed direct values fall back to
  four. All local windows count, including shells outside Git, coordinators and
  workers; `+N` is additional. Width reduces the chosen count before shortening
  long Unicode names. Selected-last sharp shrink (160→48) retains selection.
- Window indices and ordering come from the rendered local session, including a
  window linked into a non-grouped session at index17. Targets contain stable
  session/window IDs, never labels or indices. Per-row jobs receive explicit
  width/selection and hold no shared mutable rendered-row cache. Exact
  `cockpit --windows --local-session '$ID'` reaches the entire local membership,
  including mixed repositories, unassociated sessions and linked client views.
  Choosing a project explicitly leaves that local-session filter.
- The selected dot/index, role, agent identity, name, padded badge and adjacent
  tabs remain separate. Theme colours supplement labels. RUN is process-only;
  WORK requires a hook; known failing exits remain visible. Redaction and literal
  format-like names (`#[bg=red]`) preserve safe navigation. Both font modes and
  custom icons/colours work; safe mode excludes a configured font-specific icon.
- GLOBAL counts consume ticket09's pure Totals/failure/attention rules. Hidden
  failed exits with retained Review count once as attention and in both labelled
  categories. The row marks category overlap with `(overlap)`, or `*` when narrow.
  Clicks open matching global filters; `+N` opens the exact local inventory.
- Real attached-client mouse and keyboard tests cover tabs, Review, all status
  filter keys, overflow and selecting an outside-Git shell. `prefix g` enters
  the configurable status table (`f/i/r/a/w`; Escape cancels). Existing bindings
  at the default key survive installation and reconfiguration. Blank Rust-bar
  cells are inert; native/legacy status clicks retain their fallback. Other
  clients and attention remain unchanged. Vanished IDs fail rather than opening
  an index replacement; a creation between metadata and local membership reads
  makes rendering explicitly unavailable rather than miscounting overflow.

Initial public-seam tests were red for the missing status command/installer.
A future scan timestamp was separately reproduced as incorrectly fresh and fixed.
The first raw terminal observations were strengthened after review: merely
seeing GLOBAL did not establish width/theme convergence. The retained VT cell
observer now accumulates foreground/background/bold, and tests wait until both
actual rows match the requested public rendering before independently checking
labels, spacing, exact xterm-256 palette and full selected-title background.
The final blank-space/badge check deliberately separates two single clicks;
back-to-back injected clicks instead generated tmux's DoubleClick event.

### Refresh costs, freshness and limits

A first design scanned in each client's context job. With 36 workers, observer
plus two jobs and a diagnostic fresh render contended on the existing guard;
the diagnostic reached its five-second timeout. Installed rows now **reuse the
existing observer projection**: neither ambient row launches a process scan.
Tabs perform no Git probe; context asks only for the selected checkout's branch.
Explicit `status-bar` CLI rendering can still request a fresh scan; installed
scripts use `--projection`. This is not an atomic two-row snapshot: each row may
observe a different moment, while its ranges remain independently stable.

Successful complete reconciliation publishes `@drudwyn_scan_at` under the
existing lifecycle guard and child-retained mutation descriptor. Failed
reconciliation does not advance it. Missing, malformed, future and expired
values display GLOBAL STALE while retaining observed counts. Expiry is twice
`@drudwyn-interval` plus five seconds. A fixture-stopped observer shows actual
stale cells; explicit scan recovers them. No guard is bypassed, no wait-for lock,
file cache, prompt history or persistent registry is introduced. Appearance
reads retrieve only named options. No runtime path reads terminal content.

Fixture: Linux 7.1.5, tmux3.4, Git2.43.0, Cargo1.95.0, disposable `/tmp` Git
repositories and fake `codex` asserted to resolve exactly to system `sleep`.
The scale case has 36 workers across four projects and 39 distinct checkouts
(35 linked plus four primary), with 64/160-column attached clients. Latest
focused measurement: two simultaneous explicit fresh CLI renders took 1.764s
wall time (individual 1.761s / 0.969s); installed observer-backed rows for both
clients became ready in 0.969s including plugin installation. These are local
observations, not portable latency guarantees. The observer remains periodic;
unknown/expired evidence is not upgraded to fresh activity. Non-Linux runtime
has not been exercised; selected Git context uses native Linux cwd when
available and directly resolvable tmux cwd otherwise, without blanket unescaping.

### Evidence

Nine focused status cases cover the above seams. The actual convergence matrix
is 48/64/80/120/160 columns × safe/Nerd × Moon/Dawn/Rose Pine, with a differently
sized second client and a different selection. Captures were inspected:

- `/tmp/drudwyn-ticket10-{safe,nerd}-{moon,dawn,rose-pine}-{48,64,80,120,160}.{txt,ansi}`
- `/tmp/drudwyn-ticket10-cap-{1,3,4,6,auto,malformed}.txt`
- `/tmp/drudwyn-ticket10-selected-last-{48,64,160}.txt`
- `/tmp/drudwyn-ticket10-scale-{64,160}.txt`
- `/tmp/drudwyn-ticket10-stale.{txt,ansi}` and literal-format/working ANSI captures.

Focused logs: `/tmp/drudwyn-ticket10-focused.log`,
`/tmp/drudwyn-ticket10-mouse-final.log`, `/tmp/drudwyn-ticket10-convergence.log`,
`/tmp/drudwyn-ticket10-scale.log`, `/tmp/drudwyn-ticket10-stale-red.log`.
Compatibility checks passed 41 Rust tests (catalogue expectation updated from40
to42 for the two added options), 36 activity/concurrency, 10 settings, 11 global
Cockpit and 29 real-client cases. Their logs are
`/tmp/drudwyn-ticket10-{rust,activity,settings,global,clients}.log`.

Builder Standards/Spec self-review was sequential, per the coordinating
session's no-child-review instruction. It checked content-blind data flow,
projection freshness/lock lifetime, cell widths/escaping, role/count parity,
exact membership and stable actions, configuration compatibility and scope.
No ticket11 integration behavior is included. Cosmetic empty shell activity
text is recorded for ticket15 (`COORD … · ref · (unknown)` can omit the activity
segment); it does not affect counts or routing. Independent review remains
pending. Code is frozen for the final complete gates.

The first complete gate stopped at an obsolete help assertion for the previous
left/centre/right bar. The help check now verifies A's rows/controls/staleness;
an additional explicit-v1 check preserves clustered-layout guidance. That
focused check passed before restarting the frozen final gates. No renderer,
lifecycle or action changes were made for this correction.


### Final frozen-code gates

`cargo fmt --check`, `git diff --check`, `cargo test --locked` (41 tests), and
complete `bash tests/run.sh` passed. The final run includes 15 launch, 17 recovery,
11 global Cockpit, 9 status A, 36 activity/concurrency, 29 real-client, 13 navigator
(one existing optional Resurrect skip), 10 settings, and all shell/help/privacy/
package/release checks. Logs: `/tmp/drudwyn-ticket10-final-rust.log` and
`/tmp/drudwyn-ticket10-final-suite.log`.

The final 36-worker/four-project/39-checkout fixture measured 1.775s wall time
for two simultaneous fresh CLI renders (1.772s and 0.958s individually), and
0.994s until both installed 64/160-column ambient rows were ready. All actual
captures were refreshed by the passing suite. No implementation changes followed
this successful frozen run. Implementation, tests, guidance and this verified
receipt are committed together; independent review remains pending. `main`
remains `eaf24469290cbf77dd1d2a6176fbd54f7ace1868`.

## Review correction — pending independent re-review

Independent review of `2220f3f..78cd0d6` found two P2 defects. Both were reproduced
with `/tmp/drudwyn-review10-exit-label.py` and new public-command/actual-terminal
regressions before correction:

1. Local status replaced retained Review/Input with EXIT, although global totals
   still retained attention. Tabs now show attention and process receipt together,
   such as `REVIEW / EXIT 23`. Selected context attributes attention separately,
   `REVIEW (hook) / EXIT 23`; narrow context uses `REV/X23` or `IN/X0`/`IN/X?`.
   Zero remains process exit only, unknown stays unknown, and no completion or
   new hook evidence is inferred. Global count/category semantics are unchanged.
2. An empty stopped-pane cwd passed to `git -C` used the renderer's repository
   and exposed an unrelated branch. Selected context now uses the existing
   stable-pane, lossless checkout resolver. It validates pane/launch identity
   and known checkout bytes; stopped ordinary panes without that association
   show `ref ?`. The shared resolver explicitly rejects nonabsolute/empty paths
   before canonicalization. No fallback to invoking cwd or blanket display-path
   unescaping is permitted. This adds no global Git details or process scan;
   the context still performs at most one selected-branch Git query.

Two new status tests failed independently on the old implementation and now
pass. They exercise retained Review with native nonzero exit23, Input with
native zero exit, and Input with an exited/reaped child whose exit receipt is
unknown (the parent shell remains live in that last case). Converged actual
160/48-column rows assert full tab labels/padding, separately attributed context,
compact context, failure totals and absence of a completion claim. The branch
case launches a managed worker through the public command into a literal-dollar
checkout, then checks stopped binding, missing/malformed/relative metadata,
and a separate ordinary stopped pane without a checkout association. It never
uses installed agents. Source fixture still asserts fake `codex` resolves to
system `sleep`.

Captures were inspected:
`/tmp/drudwyn-ticket10-correction-{REVIEW-23,INPUT-0,INPUT-unknown}-{48,160}.{txt,ansi}`
and `/tmp/drudwyn-ticket10-correction-bound-checkout.txt`. Red evidence:
`/tmp/drudwyn-ticket10-correction-original-red.log` and
`/tmp/drudwyn-ticket10-correction-tests-red.log`. Focused green evidence:
`/tmp/drudwyn-ticket10-correction-focused.log`,
`/tmp/drudwyn-ticket10-correction-status.log` (11 cases), and
`/tmp/drudwyn-ticket10-correction-recovery.log` (17 cases, including literal
unmanaged cwd, vanished stable pane and coordinator orphan/concurrency cases).

Sequential builder Standards/Spec audit checked privacy, stable identity,
empty-path rejection, label escaping, distinct attention/exit evidence, count
parity, narrow cells, unchanged lifecycle synchronization and ticket10 scope.
No implementation changes are planned after the frozen final gates. Existing
Linux/tmux3.4 runtime limits apply; non-Linux runtime remains untested. No
integration behavior or ticket11 edits are included. Independent re-review
remains pending; this receipt does not self-clear either finding.


### Correction frozen-code gates

`cargo fmt --check`, `git diff --check`, `cargo test --locked` (41), and complete
`bash tests/run.sh` passed without a gate restart. The suite includes status11,
global11, recovery17, launch15, activity36, real clients29, navigator13 (one
existing optional Resurrect skip), settings10, and all shell/help/privacy/
package/release checks. Logs:
`/tmp/drudwyn-ticket10-correction-final-rust.log` and
`/tmp/drudwyn-ticket10-correction-final-suite.log`. Actual correction captures
were refreshed by this passing run. No implementation changes followed the
successful run. This correction commit is the fixed re-review target after
`78cd0d6b360050a0f32a3725c32ac5a47112086e`; independent re-review remains pending.
`main` remains `eaf24469290cbf77dd1d2a6176fbd54f7ace1868`.

## Independent final crosscheck receipt — 2026-09-30

Reviewed the full ticket diff `2220f3f...3c4f71f`, with correction review of
`78cd0d6...3c4f71f`, against this ticket, the approved specification, CONTRIBUTING.md,
CONTEXT.md, privacy documentation and ADRs 0006/0007. Standards and Spec were
reviewed sequentially under the implementation workflow. The independent
reviewer session was reused because of the harness thread limit; it reported
both original P2s and did not implement the ticket or its corrections.

Standards: no blocking documented-standard violation or new material smell.
Ambient rows reuse fixed observer metadata without per-tab Git/process probes,
content inspection or persistent caches. Reconciliation freshness is published
only after successful guarded writes, with mutation-child lock ownership
preserved. Selected-branch lookup now reuses the existing stable-pane resolver,
rejects empty/relative identity and respects lossless encoded checkout paths.
The renderer keeps labels separate from stable action IDs and escapes tmux
format characters; client navigation uses the existing validated routing seam.

Spec: no outstanding ticket-10 finding. The two informative rows retain stable
local ordering, selected visibility, density controls, spacing, role/activity
labels, explicit stale projection and shared global attention rules. Overflow
targets exact local session membership, including shells; mouse and keyboard
routes preserve the other client's selection and reject vanished targets.
Both P2s are corrected: retained handoff and process exit are visible together
locally, including compact narrow context, and missing checkout identity cannot
borrow the renderer's branch. Known literal checkout metadata resolves the
worker's own branch; unavailable evidence remains unknown. Attention totals,
overlap semantics, redaction, settings and explicit legacy fallback remain
consistent. The previously recorded shell-text and header grammar polish items
remain nonblocking; later integration tickets were not treated as missing scope.

Independent evidence: all nine original status cases passed on `78cd0d6` in
30.651 seconds (`/tmp/drudwyn-review10-status.log`), exercising actual terminal
cells, widths/themes/icons, caps, client actions, freshness and compatibility.
On frozen `3c4f71f`, both added correction cases passed in 5.062 seconds
(`/tmp/drudwyn-review10-correction-tests.log`): actual 48/160-column rows retain
Review/Input alongside known nonzero, zero and unknown exit; managed
literal-dollar checkout, missing/malformed/relative metadata and ordinary
stopped-pane branch context remain truthful. The original independent
`/tmp/drudwyn-review10-exit-label.py` was rerun unchanged after its fixture cwd
was pinned in the original review. Output in
`/tmp/drudwyn-review10-correction-original.log` confirms both public rendering
and installed rows show `REVIEW / EXIT 23`, `REVIEW (hook) / EXIT 23`, and
`ref ?` for the stopped ordinary worker, with unchanged global totals. All
fixtures used disposable Git/tmux resources and verified fake agents only.

The builder's frozen fmt receipt and final Rust/full-suite logs were inspected,
including 41 Rust tests, all 11 final status cases, recovery/activity/client
coverage and the existing optional Resurrect skip. Those broader gates were
not redundantly rerun or claimed as reviewer-owned full-suite evidence.
`git diff --check` passed; the tree was clean before this receipt and `main`
remains `eaf24469290cbf77dd1d2a6176fbd54f7ace1868`. Runtime evidence remains
Linux/tmux 3.4; non-Linux fallback was not exercised. This commit changes only
the ticket status and independent review receipt.
