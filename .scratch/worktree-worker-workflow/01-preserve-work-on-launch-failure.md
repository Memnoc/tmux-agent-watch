# 01 — Preserve work on launch failure

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** done

**What to build:** A failed start returns a truthful error and an actionable
path to retained resources instead of removing work the worker may have created.

**Blocked by:** None — can start immediately.
**Priority:** P0. **Stories:** 6, 7, 21, 22.

- [x] Reproduce immediate zero/nonzero exit, metadata attachment failure, and
  a worker that writes a file or commit before failing through the launch action.
- [x] Distinguish resources never used by a process from resources it may have
  modified. Retain uncertain, dirty, or newly committed work without force deletion.
- [x] Display the launch failure and retained worktree/branch identity; do not
  show success because creation alone or a process exit succeeded.
- [x] Retrying cannot overwrite existing branches/paths or duplicate a surviving
  live worker. Correctable failures leave unrelated windows/checkouts untouched.
- [x] Verify preservation through real Git state, including untracked files;
  adapt existing rollback expectations only where the spec intentionally changes them.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.

## Implementation receipt

- `workspace start` retains the branch, checkout, and any surviving worker once
  tmux may have started it, including metadata failures and ambiguous command
  responses. Only a clean, unchanged allocation whose tmux executable never
  started can be removed automatically; cleanup uses no force deletion.
- Immediate zero/nonzero exits fail startup with `remain-on-exit` both off and
  on. Known dead-pane exit status is reported without claiming task completion.
  Errors identify retained resources; Cockpit wraps errors and hides details
  under label redaction. Existing branch/path collisions refuse retries.
- Added `tests/start_failure_test.sh` at the real command/Git/tmux seam, covering
  untracked writes, commits on a clean checkout before failure, live writers after
  metadata failure, ambiguous `new-window` failure, missing worker executable,
  both exit modes, retries, unrelated resource preservation, and unused cleanup.
  The metadata fixture now injects only command failure around real tmux.
- Red/green: the initial worker-write regression failed with `failed start printed
  a success path` (rollback printed branch-deletion output). It now preserves
  the untracked file and checkout. The dead-pane regression failed with
  `start reported success for work/exit-on-0`; it now reports failure. The
  Cockpit rendered-buffer test failed because the retained path was clipped;
  it now shows path/branch and verifies redaction.
- Checks: `cargo fmt --check` passed; `cargo test --locked` passed (33 tests);
  complete `bash tests/run.sh` passed, including the new regression. Navigator
  reported 13 tests with one existing skipped test; settings reported 10 passed.
  Disposable tmux socket tests required approved sandbox escalation.
- Actual UI exercise: opened Cockpit's start form in a disposable 120x35 tmux
  terminal, typed a task, and launched a controlled worker that wrote a file and
  exited 7. Captured rendered output showed launch failure and retained
  worktree/branch; the worker's untracked file survived.
- Crosscheck: the coordinating session will run a fresh reviewer on this commit;
  no review result is claimed in this builder receipt.
- Limits: startup still uses the existing 150 ms observation frame. Later exits,
  persistent exit evidence, delivery, and recovery actions belong to subsequent
  tickets. A vanished pane cannot supply a historical exit code. These fixes
  reproduce controlled launch failures, not a proven historical field incident.
