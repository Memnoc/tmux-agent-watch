# 13 — Verify the assembled checkout

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** done

**What to build:** Users launch explicitly selected checks in the integration
checkout and see truthful revision-specific results separate from worker reports.

**Blocked by:** 11.
**Priority:** P1. **Stories:** 19, 21, 22.

- [x] Offer a visible verification action for user/coordinator-selected checks
  in the target checkout; do not guess commands or execute automatically on merge.
- [x] Associate live results with check identity, checkout, tested revision,
  timestamps, and exit status, without retaining command output or task content.
- [x] Distinguish reported worker checks, Not verified, Running, Passed, Failed,
  and stale/missing evidence; worker Review never manufactures a check result.
- [x] Failed/interrupted checks leave integrated work intact and allow rerun;
  a revision/observed working-tree change invalidates the current-result claim.
- [x] Include an untracked application input that makes assembled checks fail
  despite clean worker branches; show the tested environment's limits accurately.
- [x] Losing tmux metadata resets unprovable verification status, while Git
  ancestry can still establish integration independently.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.

## Implementation receipt — 2026-10-01

Built from fixed start `32ac200d981ca8bbd6a499df099917ec358242ba` on
`work/worktree-worker-workflow`. Main remains
`eaf24469290cbf77dd1d2a6176fbd54f7ace1868`.

- `workspace verify --path CHECKOUT` inspects evidence;
  `--check ID --command-stdin` explicitly runs a selected Bash program with
  inherited visible output. Cockpit **V** offers destination/check/command fields,
  **F5** runs visibly, **F6** inspects, and **Esc** cancels. A validated batch supplies
  its actual destination; missing associations require an explicit checkout.
- The latest live receipt per canonical checkout contains check identity, tested
  commit, Unix start/end timestamps, exit state, process lifetime and an opaque
  Git/stat fingerprint. No commands, file contents or output enter a file, log,
  runner argument, tmux option, or durable registry. Child output belongs to the visible
  terminal and is never captured by the supervisor.
- Checks have distinct missing/Running/Passed/Failed/stale evidence; worker reports
  remain explicitly unknown and Review is never proof. Observed revision, branch,
  inode or tracked/untracked metadata drift permanently invalidates a receipt.
  Git ancestry remains independently inspectable after metadata loss.
- The child runs through the held directory descriptor and retains its inode
  guard separately from transient command stdin. Killed/replaced runners become
  interrupted; surviving foreground children block competing operations. Git
  namespace and Bash startup/history overrides are removed. No work is reset.
- Inventory shares one receipt observation per actual batch destination per
  refresh. Missing receipts skip file enumeration; Git/stat work occurs outside
  the lifecycle guard, with receipt comparison before writes. Redaction covers
  check identities, paths and revisions, including the 48-column form.

Test-first evidence: the first public regression failed because `workspace verify`
was absent (no Not verified output); the next real-terminal regression failed
waiting for the absent ASSEMBLED VERIFICATION form after **V**. Both passed after
implementation. The initial sandbox attempt could not open an isolated tmux
socket; all behavioral runs used authorized disposable tmux/Git fixtures outside
that socket restriction, with verified fake agents.

Nine focused public-command/real-terminal cases passed in 16.390s before the
final gate. They cover explicit failed checks caused by an untracked application
input despite a clean worker branch; successful rerun; same-size repeated dirty
edits and observed-then-reverted changes; revision movement; running/overlapping
checks; killed runner with surviving locked child; receipt loss during/after a
check; malformed and replaced process evidence; namespace/startup isolation;
actual elsewhere checkout and distinct receipts; replaced checkout directory;
visible Cockpit execution with both clients unchanged; and narrow redaction,
cancellation and missing batch selection.

Implementation self-crosscheck: Standards and Spec were checked against
CONTRIBUTING, CONTEXT, privacy, ADR0002/0005/0006/0007 and the approved spec/plan.
No unresolved finding in the implementation review. Final code includes the
Cockpit stale-snapshot guard for **V**, process birth ticks on Linux, receipt
schema validation, and immutable receipt comparison at completion. The
coordinating session owns the independent fixed-SHA Northstar crosscheck after
this implementation commit; this self-check does not claim independent review.

Runtime evidence is Linux/tmux 3.4 only. Ignored files, external services,
symlink targets outside the checkout, nested repository contents, detached
background work, and changes between observations remain explicitly uncertified.
Commands consume stdin as a script; selected checks requiring other input must
redirect it explicitly. No checks run automatically on integration.

The first full run reached integration20 and conflict17, then exposed a new
terminal fixture's immediate **V** during the refresh triggered by **Esc**. The
captured frame showed the intended action refusal. The test now waits for the
actual Integrate frame before Escape and the ready snapshot before V, within the
existing bounded timeout. The missing-batch case also starts a fresh unredacted
app because configuration is loaded at startup. Production behavior and timeout
were unchanged; no assertion was weakened. The failed run remains at
`/tmp/drudwyn-ticket13-full-suite.log`. Both corrected terminal tests passed in
3.410s, then all nine focused cases passed in 16.349s at
`/tmp/drudwyn-ticket13-focused-final.log`.

### Review corrections and compatibility expectations

The next run passed verification9 but stopped at three older global-detail tests
that expected the placeholder `Checks: unknown`. The rendered details correctly
showed separate assembled Not verified and reported-worker unknown labels. Those
assertions now require both labels, retaining their original identity, scrolling,
redaction and metadata-loss assertions. Corrected Global11 passed in 29.038s:
`/tmp/drudwyn-ticket13-corrected-global.log`.

Independent reviewer13 reproduced a malformed completed receipt still displaying
Passed when runner/child binding shapes or timestamp ordering were corrupted.
The expanded public test reproduced seven malformed-success variants before the
fix. Validation now requires supported positive PID/birth shapes, bounded ordered
timestamps, valid check labels and consistent exit/end fields. Completed valid
bindings need no current process; missing child birth remains Not verified with
its observed exit preserved. The corrected malformed case passed in 2.128s.
Reviewer reproduction: `/tmp/drudwyn-review13-receipt-repro.py`.

A final checkout-identity regression proved that trimming all final newline bytes
from Git's root output could run a selected check in ordinary sibling `repo`
instead of actual `repo\n`, including when selected through a symlink alias.
Both variants failed by detecting the command's marker in the wrong checkout.
The fix strips exactly Git's one record terminator and rejects control characters
in caller, raw resolved and canonical paths, consistent with integration's path
contract. Both variants passed in 0.759s. No sibling or selected checkout marker
was created after the correction. Reviewer13 received this exact delta.

The earlier correction full run was already in flight during that last safety
fix and is not final frozen evidence. The final complete suite is run only after
that disposable run exits, avoiding concurrent full-suite interference.

### Final frozen gates

- `cargo fmt --check`: passed.
- `cargo test --locked`: 41 passed; no failures. Log:
  `/tmp/drudwyn-ticket13-frozen-rust.log`.
- `python3 tests/verification_test.py`: 10 public-seam tests passed in 17.049s.
  Log: `/tmp/drudwyn-ticket13-frozen-focused.log`.
- `bash tests/run.sh`: exited 0 on the final frozen code. Log:
  `/tmp/drudwyn-ticket13-frozen-full.log`. Counts: batch UI1, launch15,
  recovery17, integration20, conflict17, verification10 (17.546s), Global11,
  status11, activity39, navigator13 (one existing optional Resurrect skip),
  independent clients29, settings10, plus every legacy shell, privacy,
  packaging and release-workflow check.
- `git diff --check`: passed; generated `tests/__pycache__` removed before commit.

All gates above ran after both safety corrections. No runtime changes followed
that freeze. This implementation commit includes code, tests, documentation and
this done receipt atomically. The coordinating session owns independent final-SHA
review/receipt and subsequent tickets. No main merge, push or live user fixture
was performed; platform claims remain Linux/tmux 3.4 only.
