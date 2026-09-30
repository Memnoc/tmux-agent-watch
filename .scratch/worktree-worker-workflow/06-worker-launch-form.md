# 06 — Launch a named worker with a complete task

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** done

**What to build:** A launch form accepts a short name, editable branch, source
preview, agent, and usable task input, then creates and instructs the worker.

**Blocked by:** 01, 05.
**Priority:** P1. **Stories:** 1, 2, 3, 5, 6, 21, 22.

- [x] Short name and worker branch are independent of task prose; detailed
  instructions do not generate an oversized branch/directory name.
- [x] Edit and paste multiline instructions at supported terminal widths, or
  select a repository task-file reference; show full content/reference for review.
- [x] Validate task-file availability in the worker checkout, including the
  case where a file exists only as uncommitted planning work in the source checkout.
- [x] Launch from the displayed pinned commit, in the intended project context,
  while preserving the coordinator and other terminal's selection.
- [x] Report creation and task transmission separately; delayed startup, failed
  paste/submission, and uncertain delivery permit deliberate recovery without
  silently resending or claiming that the agent accepted/completed the task.
- [x] Bind delivery to the intended pane/process, and remove transient buffers
  on success/failure. Retain only a deliberately selected file reference, never
  task text in options, arguments, logs, or files.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.


## Implementation receipt — 2026-09-30

Implemented from `d5a6dd8` on `work/worktree-worker-workflow`.

- Replaced task-derived naming with separate short name, editable branch, pinned
  source/destination preview, agent choice, and text/file input. F6 performs the
  normal creation and single transmission action. Task prose never determines
  the branch or worktree directory. Creation stays detached and preserves the
  coordinator and existing client selections.
- Added an in-memory multiline editor with bracketed paste, cursor movement,
  line Home/End, beginning/end Page Up/Down, scrolling, Unicode column widths,
  and horizontal review of long names/branches. Tab selects fields; F4 selects
  the agent; F5 selects text or a repository task reference. Redaction hides
  task, name, branch, source, destination, and recovery labels in the form.
- Added `workspace start --task-stdin` and `--task-file`, and explicit
  `workspace deliver-task --task-file` / `--retry`. Task-file preflight uses only
  Git tree metadata at the pinned commit. Actual checkout validation requires
  a regular file resolving within that checkout; the agent reads the content.
  Uncommitted-only planning files fail before branch/worktree allocation.
- Captured window, original pane, and initial PID together in tmux's creation
  response. Retained live pane/PID/expected-command binding and not_sent/sent/
  uncertain transmission state. Active split changes, including a split created
  by the worker during initialization, cannot redirect the initial task.
  Respawned or changed processes are refused. Supported agent observation waits
  up to three seconds; a delayed/unrecognized startup preserves resources and
  leaves the task not sent. A successful paste/submission reports sent, with
  acceptance and implementation unknown.
- Kept task text on stdin and in uniquely named transient tmux buffers, including
  bracketed multiline paste. Scoped cleanup removes buffers after load, paste,
  or submission failures. An uncertain attempt requires deliberate `--retry`;
  no delivery error restarts a worker, removes its checkout, or silently resends.
  Only deliberately selected file references and operational metadata survive.
- Diagnosed ticket 05's dollar-sign path regression at the public launch seam.
  On this tmux 3.4 environment, the worker process's actual cwd was the literal
  checkout, but tmux's displayed path contained a backslash before `$` and the
  subsequent batch validation rejected that display string as a Git checkout.
  Initial batch binding now validates the known newly created checkout and
  project window membership. Launch arguments bypass the shell through direct
  argv; literal directory formats and trailing command separators are escaped.
  No global unescaping or batch-record encoding change was introduced.
- Fixed a further real terminal regression: a rapid sequence of keys followed
  by a paste larger than 1 KiB could leave the paste pending until another key.
  Non-content counters showed the application had not received the paste;
  local Crossterm 0.28.1 source identified early returns from its edge-triggered
  Mio input loop. Enabled the existing `use-dev-tty` feature for level-triggered
  polling, without a version upgrade or a custom parser. Cargo.lock adds
  filedescriptor 0.8.3 and its thiserror 1.0.69/error-derive dependencies. These
  are local descriptor/error utilities, not network or storage clients.
- Updated usage and privacy documentation. Ongoing activity/exit evidence,
  restart/resume UI, integration, and later Cockpit work remain their own tickets.

Validation:

- Red before green: `tests/batch_test.sh` reproduced the exact reported
  `Coordinator requires a Git checkout` failure for
  `suffix-worker-literal-$value`. It now confirms the original literal checkout
  through the worker process's cwd and completes all existing batch checks.
- `tests/worker_launch_test.py` supplies 11 public command/actual-key cases with
  disposable Git repositories, isolated tmux servers, and controlled receivers.
  It covers missing/uncommitted files before allocation; edited name/branch and
  multiline text; file-only reference transmission with an exact hash receipt;
  missing-file revalidation; active-pane changes and split-during-start;
  respawn refusal; injected load/paste/Enter failures; buffer removal; repeated
  send refusal; clean checkout and no task content in process arguments/options/
  environment; delayed startup and deliberate later send; and UI file mode.
- The real UI loop covers widths 48, 64, 80, 120, and 160, rapid keys followed by
  more than 1 KiB of Unicode/CJK/tab multiline paste, first/last-line review,
  single-action launch, and exact received-text hashes. The initial large-paste
  stall went red before the supported polling-backend change and is now green.
- The first width harness resolved installed Codex instead of its fake because
  setting only tmux's global/session PATH did not override the invoking client's
  PATH. Those five launches used synthetic instructions in disposable `/tmp`
  checkouts; fixture teardown killed their isolated server. They are not counted
  as verification. The corrected harness pins the caller environment and fails
  closed unless the exact fake executable resolves before launching workers.
- Final `cargo fmt --check`, `cargo test --locked` (39 passed), focused launch
  matrix (11 passed), and complete `bash tests/run.sh` passed on the final code.
  The full run includes the new launch matrix, batch and batch UI, start-failure,
  lifecycle, 13 navigator tests (one existing skip), 29 independent-navigation
  tests, 10 settings tests, privacy, installation, and packaging checks.
  `git diff --check` passed. Disposable tmux runs and the Cargo cache dependency
  download needed authorized sandbox escalation; no live user fixtures were used.

Builder Northstar check:

- Standards: reviewed the change against CONTRIBUTING, CONTEXT, privacy, and
  ADRs 0002/0003/0005/0007. Git/tmux remain authoritative; no prompt persistence,
  file-content inspection, terminal-content inspection, network client, durable
  registry, or automatic cleanup of uncertain resources was added. Input debug
  counters and generated Python cache files were removed.
- Spec: all six ticket criteria were checked against the command/UI evidence.
  Name/task separation, pinned-source file availability, ordinary single-action
  launch, original-pane/process binding, truthful transmission results, and
  deliberate recovery are covered. A sent receipt claims neither agent readiness
  nor task acceptance/completion. Existing client/coordinator behavior stays
  covered by the full suite.
- Per the coordinating session's workflow, fresh independent review follows
  this atomic implementation commit; this receipt does not claim that review
  is already complete.

Limits: process observation cannot prove a third-party agent editor is ready or
has accepted the task; the UI/CLI explicitly reports transmission only. Live
metadata and task-file associations do not reconstruct lost conversations or
persist across tmux restarts. The original literal-path fix applies to initial
launch binding and does not claim a general redesign of tmux path display.
`main` remains `eaf2446`.

## Correction receipt — concurrent delivery — 2026-09-30

Corrected the independent review's Spec P2 finding from `99e083b`: two default
delivery commands could both read `not_sent` before either marked the attempt
uncertain, then both paste, submit, and report success.

- A nonblocking kernel advisory lock on the worker's existing checkout directory
  now covers binding and delivery-state revalidation, buffer loading, paste,
  submission, receipt, and scoped buffer cleanup. Overlapping default calls and
  explicit retries are refused. Sibling checkouts remain independent; windows
  deliberately sharing one checkout share the guard conservatively.
- The lock is held by a close-on-exec descriptor and released on command return
  or process death. No lock file, owner registry, daemon, content read, or new
  dependency was added. A crashed attempt that already transmitted remains
  uncertain and requires an explicit retry; failure never triggers a resend.
- Launch records a losslessly encoded canonical checkout path in live tmux
  metadata. This avoids interpreting tmux's escaped path display as a filesystem
  identity. The guarded path's inode and the original window/pane/process binding
  are revalidated before reading delivery state. Missing or malformed launch
  identity, absent directories, and unsupported locking fail closed, with no
  global or unrelated-directory fallback. Older windows without this new live
  identity cannot use guarded delivery; `--retry` does not bypass that check.
- Usage and privacy documentation describe this operational metadata and guard.
  The older v2 privacy fixture now obtains its bound fake receiver through the
  public workspace-start command instead of creating an unbound raw tmux pane;
  its transmission and privacy assertions remain intact.

Validation:

- Red before green: `python3 tests/worker_launch_test.py
  WorkerLaunchTest.test_concurrent_delivery_sends_once_and_allows_later_explicit_retry`
  failed with `Both overlapping deliveries reported success`. The test pauses
  the first real CLI process at the tmux load boundary, invokes competing actual
  CLI processes, then checks a single receiver submission and later explicit
  retry. It passes with the guard.
- The literal `$` plus backslash checkout test also went red before lossless
  launch identity was added. It now sends successfully and proves a different
  worker can receive while the first delivery is paused.
- The 15-case launch matrix passed, including concurrent default/retry refusal,
  killed-owner recovery before load and after Enter, sticky uncertain state,
  deliberate later retry, release after injected load/paste/Enter failures,
  empty transient buffers, and missing/malformed/unavailable checkout refusal.
  New receivers use exact absolute fake-agent paths; the existing UI resolution
  checks remain fail-closed. No real agent was launched for this correction.
- The original `/tmp/review06-probe.py` now reports one successful delivery and
  one busy refusal, one receiver hash, and no remaining buffers.
- Final `cargo fmt --check`, `cargo test --locked` (39 passed), focused launch
  matrix (15 passed), focused `tests/v2_test.sh`, complete `bash tests/run.sh`,
  and `git diff --check` passed. The full run includes the same 15 launch cases,
  29 independent-navigation cases, 10 settings cases, and the existing navigator
  skip. Disposable tmux fixtures required authorized sandbox escalation.

Builder review: Standards and Spec checked against CONTRIBUTING, CONTEXT, the
privacy boundary, and ticket 06. This addresses the reported concurrency gap
while preserving initial pane/process binding, failure cleanup, explicit retry,
and truthful transmission receipts. No later ticket or specification was changed.
Fresh independent review follows this atomic correction commit.

Limits: the guard is checkout-wide and relies on filesystem advisory-lock support.
It uses Rust's [standard Unix nonblocking flock mapping](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock);
[Apple documents the corresponding flock API and unsupported-object errors](https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/flock.2.html).
Linux was exercised here; macOS runtime behavior was not exercised. Process
observation and a sent receipt still do not prove agent readiness, acceptance,
or completion. `main` remains `eaf2446`.

## Independent correction crosscheck — 2026-09-30

Reviewed `git diff 99e083b...1de76ab` against this ticket, the originating spec,
CONTRIBUTING, CONTEXT, privacy, and the stateless/tool-agnostic ADRs, with the
whole-ticket baseline `d5a6dd8` as context. No production changes were made.

- **Standards: 0 findings.** The existing-directory descriptor guard adds no
  task store, content inspection, lock file, dependency, or fallback delivery.
  Its lifetime covers state checks, transmission, receipts, and buffer cleanup.
- **Spec: 0 findings.** The original concurrent default-delivery P2 is resolved:
  default and explicit-retry competitors are refused while a delivery holds the
  checkout lock. Binding and checkout identity are revalidated; literal paths
  remain usable, independent worktrees remain independent, and failures retain
  the worker without automatic resending.
- Independently reran five focused public-command tests: concurrent default and
  retry calls; killed owner before load/after submission; literal dollar-sign
  and backslash checkout with a parallel sibling; missing/malformed/unavailable
  checkout identity; injected load/paste/submission failures and later recovery.
  All five passed. The initial sandbox run could not create disposable tmux
  sockets; the authorized escalated run passed.
- Two additional disposable review probes passed: changing the recorded launch
  PID while buffer loading is paused refuses transmission and cleans the buffer;
  assigning two windows the same checkout identity conservatively serializes
  them and allows a later send after release. These probes assert that their
  absolute fake-agent executable resolves to Python before launch. No installed
  agent was invoked. The shared-identity probe tests the lock key, not a recovery
  or shared-checkout launch feature.
- `git diff --check` passed. The builder's recorded full-suite results were
  reviewed, not rerun in this focused correction review. Runtime verification
  was Linux only; macOS directory locking remains untested here. Future recovery
  and coordinator delivery paths must establish explicit live launch binding
  rather than weaken the missing-identity guard.

The feature working tree was clean before this receipt-only change; `main`
remains `eaf2446`. This review closes the reported concurrency finding without
claiming agent readiness, acceptance, or task completion.
