# 11 — Integrate reviewed worker commits

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** done — independently reviewed through `346e42340a1b5c79c3ac586178326c742681c919`; Standards and Spec clear of material findings.

**What to build:** Users inspect and integrate commits into the chosen destination
from Cockpit or a command, without manually opening a shell to run Git.

**Blocked by:** 05, 09.
**Priority:** P1. **Stories:** 17, 21, 22.

- [x] Preview source/target refs, commit IDs, target checkout, changed-file
  metadata, and available check evidence; revalidate before applying.
- [x] Perform fast-forward or normal divergent clean merge; report already-
  contained work as a no-op. Update integration from actual ancestry, not Review.
- [x] Reject dirty/detached/ambiguous targets, active Git operations, stale
  previews, and unsafe untracked-file collisions without stashing or resetting.
- [x] Serialize Drudwyn operations on the same destination and preserve its
  existing checkout; a second client cannot concurrently start another merge there.
- [x] Keep merge failure/conflict visible and recoverable, with no success claim
  or cleanup. Coordinator resolution is added by 12; this slice must remain safe.
- [x] Test both direct-to-base and integration-branch destinations, missing
  metadata, target-branch checkout elsewhere, cancellation, and Git failures.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.


## Builder receipt — pending independent review

Implemented against `29144fd311cec9574a94e82b0bbac93baadafbb7` on
`work/worktree-worker-workflow`. The shared integration module owns preview,
revalidation, ancestry, destination serialization and merge outcomes; the public
`workspace integrate` command and Cockpit **i** cross that same seam. No ticket12
coordinator delivery/Continue/Abort or ticket13 verification behavior is included.

### Verified acceptance evidence

- Preview is read-only and cancellation preserves refs/files. It shows full
  source/target refs and commits, canonical actual checkout, commit IDs, explicit
  target-versus-source file-name comparison (not a predicted merge result), and
  unknown checks. No file/diff/commit-body content is read. CLI apply requires the
  ephemeral token for that state; Cockpit retains the reviewed object in memory.
- Fast-forward, normal divergent merge with both parents, and already-contained
  no-op are asserted against Git ancestry. Explicit direct destination and live
  integration-branch batch paths work when the original checkout is on another
  branch and the actual target is elsewhere. Batch checkout movement requires
  deliberate reselection; neither source starting pin nor target is reset.
- Both sides reject tracked/staged/untracked changes, active merge/cherry-pick/
  revert/rebase/sequencer state and Git locks. Detached or multiply checked-out
  destinations fail. Source ref changes, source/target commit changes, and a
  recreated directory at the same pathname invalidate review. Git is invoked
  with no autostash and ignored-file overwrite protection; an ignored collision
  retains its original bytes and target commit.
- The existing target directory inode lock serializes clients and canonical
  aliases in the same namespace as recovery/delivery. The controlled mutating
  child inherits its descriptor through stdin and begins in the target directory.
  A paused real Git wrapper survives parent death while a second client is
  still refused; after release the actual Git merge completes and a fresh preview
  reports containment. Lifecycle scan remains available while Git is paused.
  A real Python pre-merge hook starts normally and reads EOF from its Git-provided
  stdin. No persistent lock file, tmux wait-for lock or global lifecycle lock is
  held across merge/hooks.
- Controlled add/add conflict and failing merge hook retain Git's merge operation,
  source checkout and target files. No success or cleanup is reported; hook output
  is suppressed rather than captured or interpreted. Another integration is
  refused until the existing operation is deliberately handled.
- Actual Cockpit previews/cancel/apply preserve both attached clients' selections.
  Stale commit review and a vanished pane/window replaced by the same name/index
  are refused. Missing batch metadata opens explicit destination selection.
  48-column preview and all33 changed names remain reachable through scrolling;
  redaction conceals paths, refs, commit IDs and names while preserving controls.
- Full details derive committed containment from current live batch targets,
  sharing destination and source-commit probes within each refresh. External
  merge changes Not contained to Contained; moving the target back changes it
  back; metadata loss becomes unknown. Retained Review is independent throughout.
  Missing targets or mismatched associations never inherit a prior success label.
  No ancestry probe is performed per render or status cell.

### Red/green work and audit

The first public command regression failed because Integrate did not exist;
Cockpit's action was also exercised before wiring. Later audits reproduced
inherited `GIT_DIR`/`GIT_WORK_TREE` redirecting the explicit source to an unrelated
repository and separately contaminating snapshot source metadata with the target
commit, falsely reporting containment. Integration and inventory now share an
explicit-checkout Git constructor that removes repository/index/object namespace
variables without reading or storing their values. Both public command and
actual-detail regressions now pass. Other Git identity/configuration and normal
hook behavior remain available.

Fixture corrections were kept distinct from implementation defects: a broad UI
search initially selected an ordinary worker with the same name; the fixture now
selects a unique managed name. An initial Python Git wrapper could not initialize
stdin from a directory FD; the orphan test uses a controlled shell wrapper and
asserts the exact system Git executable. A separate real Python Git hook passes.
Escape followed immediately by text produced an Alt key; UI fixtures now wait
for each visible mode transition. Retrying i during an actual refresh is correctly
blocked; that fixture waits for visible freshness before the next action.

Focused integration15, global Cockpit11 and existing real-client29 regressions
passed before freeze. Sequential builder Standards/Spec audit checked the
content-blind boundary, explicit path/ref identity, current ancestry, missing
metadata, lock and orphan-child lifetime, literal paths, redaction, UI scrolling,
client independence and ticket scope. Cosmetic shell-context/singular-project
polish remains reserved for ticket15. Independent review remains pending.

Evidence: `/tmp/drudwyn-ticket11-first-red.log`, `-first-green.log`,
`-environment-red.log`, `-detail-environment-red.log`, `-focused.log`, `-lock.log`,
`-details-green.log`, `-global.log`, and `-clients.log` (all with the same
`/tmp/drudwyn-ticket11` prefix). Inspected actual terminal captures:
`/tmp/drudwyn-ticket11-narrow-preview.txt`, `-narrow-bottom.txt`, and
`-redacted-preview.txt`. Fixtures use disposable `/tmp` Git/tmux servers with
fake codex asserted to resolve to system sleep. Runtime exercised on Linux with
tmux3.4/Git2.43; non-Linux runtime remains untested. Review tokens are in-memory
comparison checks, not durable approval records or authentication.


### Final frozen-code gates

`cargo fmt --check`, `git diff --check`, `cargo test --locked` (41 Rust tests),
and complete `bash tests/run.sh` passed without a restart. The suite includes
integration15, launch15, recovery17, global Cockpit11, status A11, activity36,
real clients29, navigator13 (one existing optional Resurrect skip), settings10,
and all shell/help/privacy/package/release checks. Final logs:
`/tmp/drudwyn-ticket11-final-rust.log` and
`/tmp/drudwyn-ticket11-final-suite.log`. No runtime changes followed the green run.

The existing global fixture still has 36 workers across 4 projects and 39 distinct
checkouts; final CLI snapshot wall time was 1.183s, UI ready 1.136s, End inspection
29ms and inspection during a gated refresh 28ms on this test environment. This
fixture is a regression observation, not a portable guarantee or a new
integration-specific throughput benchmark. No status-row ancestry work was added.

Implementation and verified receipt are committed atomically on the feature
branch, with independent review pending. `main` remains
`eaf24469290cbf77dd1d2a6176fbd54f7ace1868`.

## Review correction — pending independent re-review

Independent review of `29144fd..948b902` found one P2: inherited
`branch.<destination>.mergeOptions` could replace the advertised integration
mode. With `--squash`, a fast-forward preview staged the source changes but did
not advance HEAD or leave MERGE_HEAD; with `--no-commit`, divergent integration
stopped with staged changes and MERGE_HEAD. Apply reported uncertainty rather
than success, but it had still performed an unadvertised alternate operation.

Apply now explicitly passes `--no-squash --commit` before its planned
`--ff-only`/`--no-ff` mode. This keeps the preview's promised fast-forward or
committed normal merge authoritative while preserving normal Git hooks. No
other mutation, locking, preview-token or conflict behavior changed.

The new public-command regression uses real Git for eight combinations:
fast-forward/divergent histories crossed with `--squash`, `--no-commit`, their
combination, and their combination with the opposing fast-forward setting.
Seven combinations failed before the fix (plain no-commit does not prevent a
fast-forward), then all eight passed. Each verifies exact resulting HEAD or
both merge parents, an empty worktree/index status, absent MERGE_HEAD, retained
worker checkout, and a real Python post-merge hook receiving Git's normal-merge
argument `0` with usable stdin. Existing Python pre-merge, failure/conflict,
concurrent/orphan, ancestry, redaction and UI regressions remain green.

Evidence: `/tmp/drudwyn-ticket11-merge-options-red.log` and
`/tmp/drudwyn-ticket11-merge-options-focused.log` (16 integration cases).
Reviewer originals remain `/tmp/drudwyn-review11-merge-options.py` and
`/tmp/drudwyn-review11-merge-options-ff.py`. Sequential Standards/Spec correction
audit confirmed that the explicit options enforce the reviewed mode without
suppressing hooks or weakening preserved-failure behavior. Code is frozen for
final gates; independent re-review remains pending. No ticket12 changes.

### Regression-gate lifecycle correction (ticket07 behavior)

The first correction full suite passed launch15, recovery17, integration16 and
Global11, then failed the status test's worker-discovery wait before rendering.
The independent diagnostic `/tmp/drudwyn-ticket11-status-diagnose.log` reproduced
valid new-root/new-child hook evidence being erased immediately after exit.
This is a separate lifecycle race discovered during ticket11 validation, not
another integration defect: tmux sampled a live pane, then its bound root exited
and was reaped before the subsequent process sample. The old reconciliation
cleared the binding because neither the captured pane nor the process sample
proved exit coherently.

Reconciliation now checks the two observations before any projection writes.
A live pane missing its root in the process sample triggers up to three complete
resamples; persistent disagreement returns explicit retry uncertainty, retaining
prior evidence and the old freshness timestamp. Dead panes naturally permit a
missing root. Existing root/child birth identity checks still discard replacement
lifetimes; the guard and mutation-child inheritance are unchanged. Ordinary
coherent scans perform the same number of subprocesses as before. This is a
bounded consistency check, not a claim that tmux and process snapshots are atomic.

The deterministic public scan test gates an actual tmux live-pane response,
exits/reaps its controlled fake worker and shell, then permits the real process
sample. Both Review and Input were erased before the fix, and both survive as
hook history alongside exit afterward. Additional cases prove replacement in
that gap receives fresh process evidence and persistent missing-root observation
fails without changing attention or freshness. Evidence:
`/tmp/drudwyn-ticket11-lifecycle-race-red.log`,
`/tmp/drudwyn-ticket11-lifecycle-race-green.log`, and
`/tmp/drudwyn-ticket11-lifecycle-targeted.log` (three new cases).
The first full activity run passed the original36 cases, including orphan,
queued-hook, zombie/reaping and replacement coverage. Its new fixtures required
shorter socket names and explicit child reaping before root exit; corrected
fixtures then passed all three targeted cases. No production checks were relaxed.
Independent early patch inspection found the bounded pre-mutation retry clear;
final correction re-review remains pending after the complete gates below.


### Final correction gates

- `cargo fmt --check` and `git diff --check` passed.
- `cargo test --locked`: 41 Rust tests passed.
- `bash tests/run.sh`: the complete frozen suite passed, including launch15,
  recovery17, integration16, Global11, status11, activity39, clients29,
  navigator13 (one existing optional Resurrect skip), settings10, and the
  lifecycle shell, privacy, packaging and release-workflow checks.
- Final logs: `/tmp/drudwyn-ticket11-correction-final-rust.log` and
  `/tmp/drudwyn-ticket11-correction-final-suite.log`. Focused actual status
  evidence: `/tmp/drudwyn-ticket11-status-focused.log` (11 cases).
- Sequential Standards/Spec audit: no new content observation, persistent state,
  hook suppression, destructive Git cleanup, or weakened lifetime checks. The
  observation retry completes before any window mutations or scan freshness
  publication. Validation uses Linux, real Git/tmux and controlled fake agents;
  no installed agents or live user projects were used. Cross-platform runtime
  behavior beyond this environment remains unverified.

Both corrections and this receipt are committed together. Independent correction
re-review is pending; this receipt does not self-clear it. `main` remains
`eaf24469290cbf77dd1d2a6176fbd54f7ace1868`. No ticket12 work was started;
work pauses after ticket11 independent clearance as requested.

### Independent final crosscheck receipt

Reviewed fixed full-ticket range
`29144fd311cec9574a94e82b0bbac93baadafbb7..346e42340a1b5c79c3ac586178326c742681c919`,
including correction range `948b902..346e423`. This reused reviewer session did
not implement either correction. Standards and Spec were reviewed sequentially
under the coordinating implement-all protocol, without nested reviewers.
Sources were CONTRIBUTING, CONTEXT, privacy, ADR0005, the approved workflow spec
and ticket plan, this ticket, and the crosscheck skill's standards/smell baseline.

Standards: no new material finding. The shared integration seam keeps CLI and
Cockpit behavior aligned, observes metadata rather than task/file/diff bodies,
and retains explicit errors and normal Git hooks. Canonical checkout identity,
target-directory locking with mutation-child inheritance, environment-address
isolation, and current-ancestry enrichment preserve the documented boundaries.

Spec: the original P2 is resolved. Explicit `--no-squash --commit` makes the
reviewed fast-forward or normal merge authoritative over branch merge options.
Both original independent scripts were rerun against the frozen correction:
`/tmp/drudwyn-review11-merge-options.py` and `-merge-options-ff.py`. All four
fast-forward/divergent × squash/no-commit cases now return success, advance HEAD,
leave clean status and no MERGE_HEAD, and retain the worker. Evidence:
`/tmp/drudwyn-review11-merge-options-final.log` and
`/tmp/drudwyn-review11-merge-options-ff-final.log`.

The narrowly related lifecycle correction was independently inspected and its
three deterministic public-scan regressions rerun: exit between observations
preserves Review/Input and their timestamps; replacement does not inherit old
attention; persistent incoherence preserves prior evidence and freshness while
returning retry uncertainty. All three passed in 2.358s; evidence:
`/tmp/drudwyn-review11-lifecycle-final.log`. The retry remains bounded and does
not claim atomic tmux/process observation or weaken identity/birth validation.

Earlier independent checks on the ticket passed five focused integration
regressions in 5.031s: orphan-child/alias serialization, conflict and failing-hook
preservation, stale/vanished Cockpit identity, ignored-file collision, and Git
environment addressing. Evidence: `/tmp/drudwyn-review11-focused.log`.
Actual narrow preview/end captures were inspected. No additional blockers were
found in preview revalidation, actual destination selection, ancestry/no-op,
stable UI targeting, redaction, client independence, or preserved failures.
Unknown checks and later coordinator handling remain correctly scoped to 13/12.

Builder frozen-gate logs were inspected rather than duplicating the full suite:
`/tmp/drudwyn-ticket11-correction-final-rust.log` (41 passed) and
`/tmp/drudwyn-ticket11-correction-final-suite.log` (complete suite green,
including integration16/activity39; one existing optional Resurrect skip).
Reviewer fixtures used disposable temporary Git/tmux servers and asserted fake
agents only. Runtime evidence is Linux with tmux3.4; other platforms remain
unverified. No production edits, push, or ticket12 implementation were made by
this review. The final receipt and pause checkpoint are documentation only;
`main` remains `eaf24469290cbf77dd1d2a6176fbd54f7ace1868`.
