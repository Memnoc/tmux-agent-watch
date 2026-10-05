# 28 — Finish startup handling and clear up remaining labels

Status: complete
Spec: docs/specs/2026-10-04-worktree-feedback-comparison.md
Review baseline: bf8ae52 (before this implementation)

## Authorized scope and acceptance

The user requested all three remaining issues from the October 5 live test:
1. Initial task must start without manually submitting its pasted text.
2. Default branch must retain the same recognizable name across setup/preview.
3. No-test-yet messaging must explain the next step without technical warnings.

First-run Codex setup remains a user decision. Drudwyn must not answer that
screen. Once the user finishes setup, a pending task should submit once without
repasting or another Enter. Trusted repositories should start directly.

## Root-cause evidence and implementation

The previous raw-input fixture modeled delayed reads, but not first-run dialogs.
An isolated Codex 0.160.0 with a local dummy provider reproduced the visible
symptom: automatic delivery left the text in the editor; manual Enter started it.
Changing only the disposable repo's explicit trust entry made automatic delivery
work. A minimized controlled gate also showed the old sender answering setup.
This establishes a matching mechanism, not a capture of the unseen original
startup dialog in the user's earlier run.

Both Codex screens use raw input; setup hides the cursor and the editor shows it.
Startup now requires stable editor mode (including that non-content cursor flag).
After the short readiness window, a Codex helper can wait up to two minutes with
task text held solely in stdin/process memory. It holds the existing checkout
lock, revalidates pane/process/checkout identity, and sends once after setup.
Closed/replaced workers or changed checkout identity discard the pending task.
Live helper lifetime metadata exposes interrupted/expired waits and a deliberate
restart action. The two-minute deadline bounds waiting for setup, including
stalled metadata calls. Normal transmission marks uncertainty before leaving
that waiting phase, so its later interruption cannot falsely claim not sent.
There is no automatic retry after partial/uncertain transmission. No screen
capture, prompt file, persistent registry, trust bypass, or user config edit is
introduced. Current protocol checks are validated against Codex 0.160.0; the
cursor flag is an observed readiness signal, not a general acceptance protocol.

CLI, Cockpit and recovery distinguish waiting from sent. The task remains
transient: after timeout/interruption it must be explicitly supplied again.

The source form shows the resolved local/upstream branch as its default instead
of an editable base token. An empty source retains default resolution semantics;
CLI base/current tokens remain compatible. Preview uses the same branch label.
The no-receipt UI message says Tests haven't been run here yet in a neutral
color. Other stale/failed/unavailable checks stay distinct; Details/CLI preserve
technical evidence and worker-check separation.

## Tests

Approved seams remain public CLI, disposable Git/tmux, actual keys and rendered
buffers. The opt-in installed-Codex test uses an isolated CODEX_HOME and a local
HTTP dummy provider; no account credentials or real model work is used. It
observes exactly one synthetic task request, distinguishing Codex's separate
title-generation request, and cleans up its terminal/processes.

Red: old sender answered the synthetic SETUP GATE. Old default source showed
base while preview showed trunk. Old merge preview exposed missing live evidence.
Green checks cover those paths, trusted/untrusted real Codex submission, pending
worker replacement, ordinary delivery guards, recovery, and branch/check wording.
Final test and review results are recorded below.

## Independent review corrections

Standards found that a pending worker could change its real cwd and that blocked
metadata could outlive the stated setup wait. Spec found the renamed/replaced
checkout case and stale waiting promises after helper death. Regression tests
reproduced the wrong-folder and replaced-folder sends against the old binary.

The helper now validates the held directory inode and actual worker cwd before
sending, bounds setup observation with its own process group watchdog, and bounds
its initial acknowledgement. Non-content helper PID/birth/deadline metadata lets
Cockpit derive interrupted, expired or changed-worker guidance without storing
tasks. A killed helper cannot leave a promise of future submission. Normal
transmission records uncertainty before leaving the setup-wait phase; an expired
setup timestamp cannot later claim that an in-progress transport never sent.
A stalled-transport regression also proves overlapping retry is refused.

Final rechecks: Standards — no remaining blocking findings. Spec — no remaining
findings. Both reviewed the corrected working diff; validation is recorded below.

## Final validation

- `cargo fmt --check`, `git diff --check`, and all 43 Rust tests passed.
- Complete `tests/run.sh` passed with the installed-Codex opt-in enabled:
  `/tmp/drudwyn-core-stable-suite.log`. This covers real Git/tmux launch,
  recovery, merge/conflict/check/cleanup routes, global Cockpit, rendered UI,
  status layouts, lifecycle, independent clients, privacy and packaging/runtime
  checks, then both assembled workflows using the release build. The optional
  tmux-resurrect save-script test was skipped because its script was not supplied.
- Real Codex 0.160.0: trusted and first-time setup routes each submitted exactly
  one synthetic task to the local dummy provider. No live account/model work.
- Five pending-task lifecycle cases passed in the full suite, including a real
  two-minute deadline with deliberately blocked metadata. A sixth regression
  for wall-clock expiry against a still-live helper was added during final
  review, reproduced red, then passed separately after its fix:
  `/tmp/drudwyn-wait-clock-{red,green}.log`. Rust checks were rerun afterward.
  Matching helper liveness now takes precedence over wall-clock expiry.
- Final independent Standards and Spec rechecks: no remaining blockers.

An earlier full run exposed an assertion for the intentionally replaced warning
text; it was updated to the new user-visible behavior. A subsequent attempt
interrupted one launch while the executable was concurrently rebuilt; the final
run used the stabilized implementation. The final clock-label correction was
verified separately before the suite's remaining checks and release build.

Validation was performed on this Linux/tmux setup. Other Unix metadata paths are
preserved; this receipt does not claim a new macOS live test. The existing user's
successful merge/check repeat remains the manual acceptance evidence. The new
startup fixture specifically adds the previously missing real-editor acceptance.
