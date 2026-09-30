# 10 — Ship status-bar A with density controls

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** done

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
