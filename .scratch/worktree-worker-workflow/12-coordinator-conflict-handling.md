# 12 — Resolve integration conflicts through the coordinator

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** done

**What to build:** A conflicting integration has a visible agent handoff and
Continue/Abort controls in Drudwyn.

**Blocked by:** 11.
**Priority:** P1. **Stories:** 18, 21, 22.

- [x] Create a controlled merge conflict and leave it in the correct destination
  checkout with source/target identity and an explicit conflict state.
- [x] Route a transient instruction to the coordinator agent in that checkout;
  expose Open/recovery/retry when it is absent or delivery is uncertain.
- [x] Continue requires resolved unmerged entries and the expected operation;
  it cannot complete an unrelated merge or imply that assembled checks passed.
- [x] Abort uses Git's normal behavior and preserves work on failure; do not use
  hard reset or silently discard resolution edits as a fallback.
- [x] Reconcile if the coordinator completed/aborted independently; repeated
  actions do not duplicate commits or resend tasks without deliberate retry.
- [x] Drudwyn inspects Git metadata only; conflict contents and resolution are
  handled by the coordinator agent, not terminal scraping or a hidden merge solver.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.


## Implementation receipt — 2026-10-01

Implemented from fixed base `a0075def1b75fc7e2f053d59cf0537f6b3353bc9` on
`work/worktree-worker-workflow`. The interrupted changes were inspected and
resumed; the first six retained public CLI tests passed before further slices.

A retained conflict has a live tmux receipt containing source/target refs and
commits, canonical destination identity, MERGE_HEAD inode/timestamps, project,
and handoff state. Continue and normal Git Abort revalidate that expected
operation under the shared destination guard; their Git child retains the guard
after supervisor death. External completion/abort reconciles from current Git,
and replacement/lost receipts cannot authorize another operation. Continue
explicitly supplies a noninteractive editor without suppressing normal hooks.

The initial handoff and deliberate Retry share the transient worker-task buffer
channel. An ordinary coordinator gets its own verified unique pane/process-birth
and actual-checkout binding; managed launch guards remain intact. Uncertainty is
recorded before paste, buffers are removed, and inspection never resends. Explicit
Recover agent creates a fresh destination conversation without a task, preserves
the old coordinator window/branch, and compares coordinator ownership under the
existing lifecycle guard before selecting it. Retry then supplies the generated
instruction once. Missing routing can be repaired by explicitly selecting a
validated project; no project/batch is guessed.

Cockpit automatically shows the active conflict from the reviewed attempt, with
Continue/Abort/Open/Recover agent/Retry/Refresh and explicit project selection.
The standalone `C Conflict` action revisits it. Real 48-column redacted output,
recovery-to-retry keystrokes, and two-client navigation were exercised. Inspected
captures: `/tmp/drudwyn-ticket12-narrow-redacted.txt` and
`/tmp/drudwyn-ticket12-recovered-agent.txt`. Usage, command help, and the privacy
manifest document the new behavior and live metadata boundary.

### Regressions found during the final audit

- A historical Completed receipt could hide a later failed integration in the
  Cockpit. A real ignored-file failure reproduced the stale panel. Automatic
  transition now requires the reviewed source/target identity and an active
  expected operation; explicit inspection may still show historical completion.
- An upstream ticket11 protection gap was confirmed with real divergent Git:
  `--no-overwrite-ignore` alone allowed an ignored file's bytes to be replaced.
  A pre-mutation filename check now protects exact and both file/directory prefix
  collisions against incoming changes from every merge base. It reads no file
  contents. Fast-forward/divergent cases preserve bytes and HEAD; unrelated
  ignored dependencies and unchanged source paths deleted at the target remain
  allowed. These are preservation corrections, not new workflow scope.

### Validation and audit

- `cargo fmt --check`, `git diff --check`, and 41 Rust tests passed.
  Final Rust log: `/tmp/drudwyn-ticket12-rust-final.log`.
- All 17 public conflict command/UI tests passed: controlled conflicts, ordinary
  coordinator single-send/retry, resolution/Continue, normal Abort failure,
  external completion/abort, same-ref operation replacement, missing receipts,
  absent/ambiguous/wrong-checkout coordinators, explicit project repair, agent
  recovery, uncertain sends, process replacement after paste, inherited Git/editor
  settings, private hook output, mutation-child lock retention, client independence,
  narrow redaction, and stale Completed receipt isolation.
- All 18 integration tests passed, including the new ignored-path cases.
- The first frozen full suite passed before the two audit findings
  (`/tmp/drudwyn-ticket12-full.log`). After their red reproductions and focused
  passing regressions, the final candidate was frozen and the complete suite
  rerun successfully (`/tmp/drudwyn-ticket12-full-final.log`): launch15, recovery17,
  integration18, conflict17, Global11, status11, activity39, clients29, navigator13
  (one existing optional Resurrect skip), settings10, and all shell/privacy/
  packaging/release-workflow gates.
- Sequential implementation audit found no conflict-content observation, prompt
  persistence, hidden solver, automatic resend, destructive Abort fallback,
  verification claim, or weakening of the existing lifecycle guard/provenance.
  The actual instruction remains memory/stdin-only; sent is not acceptance.
  Tests use disposable Git/tmux and verified fake agent executables only.
  Runtime evidence is Linux/tmux3.4; other platforms remain unverified.

Implementation and required gates are complete. Independent Standards/Spec
crosscheck by the coordinating session is pending; this receipt does not
self-clear that review. No ticket13+ work, push, deployment, or main integration
was performed. `main` remains
`eaf24469290cbf77dd1d2a6176fbd54f7ace1868`.


## Independent-review correction — directory-relocated ignored outputs

Review of `9be2e544e2ad08f78648b459c95e18cfb895fa14` identified one severe
preservation gap. If the destination renamed `old/` to `new/`, Git could relocate
an incoming source addition `old/incoming` to `new/incoming`, overwriting ignored
local bytes even while returning a merge conflict. The original independent
`/tmp/drudwyn-review12-directory-rename.py` reproduction was rerun red, confirming
actual byte replacement with unchanged HEAD. This was not inferred from a merge
exit status alone.

The destination guard now checks conservative directory-relocated output paths
in both directions against ignored filenames, in addition to direct outputs.
For each merge base, name-only diffs with rename detection disabled provide
removed and added/changed paths. Nonempty deleted-directory ancestors form
possible origins, and changed-directory ancestors form possible destinations.
Incoming source changes are mapped through target-side candidates; target
changes are mapped through source-side candidates. Root is never a move origin;
flattening a real directory into root remains covered. Exact, parent-file and
child-directory collisions are refused before Git mutation.

This deliberately does not predict Git's rename resolution, read file bodies,
calculate similarity, invoke a hidden merge preview, or depend on Git rename
limits/thresholds. Ambiguous possible collisions can conservatively refuse a
merge even when Git would choose another path. Usage/privacy documentation
states that limitation. Unrelated ignored paths outside candidate outputs still
allow normal integration.

Validation:

- The original independent reproduction passes after the correction: ignored
  bytes, HEAD and clean Git state are preserved; no merge is started.
- Three focused public integration tests passed in 5.916s, including ten cases
  covering both rename directions with exact/ancestor/descendant/nested/flattened
  outputs, split/partial moves, differing diff/merge rename configuration and
  branch similarity options. Removing only the collision allows normal merges
  while unrelated ignored dependency bytes remain intact.
- Independent reviewer12 reported its correction checks passed in 7.757s and
  provisionally cleared Standards/Spec, pending the final committed SHA and gates.
  The coordinating session owns the final review receipt.
- `cargo fmt --check`, `git diff --check`, and 41 Rust tests passed. Rust log:
  `/tmp/drudwyn-ticket12-rename-rust.log`.
- The first correction full run reached integration20, then exposed a UI-test
  synchronization race: immediately after resizing, the old frame could be
  cropped before Cockpit drew its footer. A bounded real-terminal probe confirmed
  the complete settled frame (`/tmp/drudwyn-ticket12-resize-settled.txt`). Only the
  test was changed to wait for that footer, retain the existing timeout, and
  separately assert redaction. Three fresh-fixture repetitions passed in 8.193s;
  no runtime UI behavior or assertion was relaxed. Reviewer12 received that exact
  test-only diff.
- The final frozen complete suite passed:
  `/tmp/drudwyn-ticket12-rename-full-final.log`. Counts: launch15, recovery17,
  integration20, conflict17, Global11, status11, activity39, clients29,
  navigator13 (one existing optional Resurrect skip), settings10, and every
  shell/privacy/packaging/release-workflow gate. The earlier failed fixture run is
  retained as `/tmp/drudwyn-ticket12-rename-full.log`.

The correction, tests, documentation and this receipt are committed together.
All fixtures were disposable Git/tmux with verified fake executables; runtime
coverage remains Linux/tmux3.4. Independent final-SHA confirmation is pending.
No ticket13+ work or push was performed; `main` remains
`eaf24469290cbf77dd1d2a6176fbd54f7ace1868`.
