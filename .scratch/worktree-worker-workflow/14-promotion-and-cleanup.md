# 14 — Promote integration branches and clean up safely

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** done

**What to build:** Users explicitly merge an integration branch into the base,
then remove eligible worker worktrees without losing branches or active work.

**Blocked by:** 12, 13.
**Priority:** P1. **Stories:** 20, 21.

- [x] Offer explicit promotion with a fresh source/target preview using the
  same merge/conflict/verification controls; never auto-promote on worker completion.
- [x] Verification on the integration branch is not copied as proof of a new
  base merge; expose the base destination's own verification state.
- [x] Finish requires a clean worktree, containment in the chosen destination,
  and no unresolved Git operation or active writer; unrelated primary HEAD
  ancestry alone is insufficient.
- [x] Confirm removal and preserve the branch; close only windows for the removed
  worktree and preserve the coordinator and other linked windows/sessions.
- [x] Dirty/untracked/unmerged work, detached checkouts, moving branch tips,
  removal failures, and repeated requests remain safe and understandable.
- [x] Neither route pushes, deploys, deletes branches, or implies shipment.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.


## Implementation receipt — 2026-10-01

Built from `6f69f7fd719d6bdd4b359f0b52890687b8dfabf5` on
`work/worktree-worker-workflow`. Runtime/test implementation frozen after the
13-case focused run; independent review belongs to the coordinating session.

The first public promotion regression failed because `workspace promote` was
absent; the first cleanup regression reproduced actual removal authorized only
by unrelated primary HEAD ancestry. Both now pass. The real Cockpit regression
first failed on absent **P**; it now proves explicit promotion and **f** removal
through the selected worker's own batch with the branch and coordinator retained.

`workspace promote --path ASSEMBLY --base BASE` shares the exact integration
preview/apply and conflict controls. Its preview/result and Cockpit form expose
the base checkout's own verification evidence. No result transfers from assembly.
Finish has preview/token/apply and interactive confirmation, an explicit batch
or destination (`--base` alias retained), current committed containment, clean
tracked/untracked/ignored files, operation checks and observed-writer vetoes.
The existing shortcut keeps its binding, uses the current pane's batch when
present, and otherwise explicitly reviews its configured base.

Source/destination directory guards survive in the Git removal child. Removal
never forces, resets or deletes a branch. Window cleanup revalidates IDs,
process births, cwd inodes and all-pane ownership. It holds the lifecycle guard
only after Git completes, and a server-side conditional rechecks pane set,
current command/cwd where available, coordinator roles across sessions, and
session last-window status. Injected split, respawn, coordinator assignment and
last-session changes after client observation all preserve those windows.
Mixed/coordinator/caller/last-session windows remain visible even if one shell
needs to change out of the removed directory.

Writer observation covers associated pane/descendant processes and readable
outside cwd metadata. Calling agents/editors are checked; only the invocation
and contiguous shell ancestors are exempt. An initial blanket same-user unreadable
cwd refusal proved unusable with nondumpable `systemd`, PAM and `ssh-agent`
session services. The agreed scoped rule vetoes unavailable associated metadata
without classifying unrelated protected services as workers. A real nondumpable
associated-worker regression then found that replaced launch lifetimes could lose
that association: prior encoded checkout identity now only vetoes deletion, never
authorizes window closure. Both associated refusal and unrelated allowance pass.
Linux uses `/proc`; hosts without it use metadata-only `lsof`, refusing if it is
unavailable. Runtime evidence remains Linux/tmux 3.4; global writer absence,
unobservable outside processes, future external mutations, and other platforms
are not certified. The preview and usage guide explain the observation scope.

Two final regressions were diagnosed before freeze. Native tmux display output
quotes `$`, while its internal comparison uses the original cwd value. The final
server guard compares authoritative raw process cwd to the internal value;
no display label is unescaped into filesystem authority. Literal-dollar linked
window closure passes. The old v2 fixture selected `v2:work-privacy` after managed
naming changed, silently resolving the wrong window and leaving the actual fake
agent alive. Its lookup now uses exact branch metadata and a stable window ID,
then deliberately replaces that verified fake worker with an idle shell. The
writer refusal was correct; it was not weakened. The old test expectation allowing
unrelated primary HEAD cleanup now explicitly requires refusal and preservation.

Shared integration path parsing strips exactly one Git record terminator and
validates the canonical path, matching verification's boundary. Direct and alias
newline-checkout regressions preserve both the selected and ordinary sibling
checkouts across Integrate, Promote and Finish. No source file contents, argv,
agent output, task history or durable cleanup registry were introduced.

Focused evidence: `/tmp/drudwyn-ticket14-focused-final.log` — 13 cases passed in
18.751s. Compatibility: `/tmp/drudwyn-ticket14-v2-final.log` — v2 shell gates
passed. Narrow real-terminal capture:
`/tmp/drudwyn-ticket14-narrow-promotion.txt`. The comparison diagnostic is retained
at `/tmp/drudwyn-ticket14-closure-format.txt`; temporary runtime debug output was
removed before the passing focused run. Final complete gate results follow.


### Independent-review correction — active shell builtins

The first frozen full run is superseded. It passed the new 13 cases but reached
an older Global test that expected the removed `FINISH WORKSPACE` modal. That
regression now deliberately exits the known source pane, selects `main` in the
new form, waits for its actual preview, replaces the selected window with a
same-name live window, and requires the original stable-identity refusal. Its
branch/checkout/replacement-preservation assertions remain. The corrected focused
Global case passes; it does not relax production selection guards.

Independent review also found a severe active-writer gap: a Bash root waiting in
a builtin `read` loop has no child process but can hold pending file writes.
The unchanged reviewer reproduction was rerun red at
`/tmp/drudwyn-ticket14-shell-red.log`: preview succeeded, Finish deleted the tree
and killed the writer's window. Executable name alone cannot prove idle. The
root-shell exemption is removed. Users must explicitly stop other worker shells
as well as agents before Finish; no shell hooks or new lifecycle inference were
added. Only the synchronous invoking command's contiguous shell ancestry is
exempt, and its background descendants are still checked even after changing cwd.
Reaped metadata probes are distinguished from live unreadable associated processes.

The unchanged independent reproduction now refuses preview/removal and retains
both checkout and active writer window:
`/tmp/drudwyn-ticket14-shell-green.log`. New public regressions cover noninteractive
and interactive builtin-only wait loops, preserve their pending writes, and prove
that an invoking shell's outside-cwd background process also vetoes removal.
Red logs: `/tmp/drudwyn-ticket14-builtin-red.log` and
`/tmp/drudwyn-ticket14-background-red.log`. Cleanup fixtures now use explicitly
exited panes with known checkout identity; mixed/coordinator/linked/session and
injected split/respawn/role guards remain asserted. V2 compatibility still passes
using its exact stable worker ID and an explicitly ended pane. A stale fixture
expectation for an exit count after replacing the managed process was corrected
to use the all-window inventory; no exit provenance was manufactured.

Usage/privacy documentation now explicitly requires stopping other source shells.
The earlier receipt's reference to an idle source-shell fixture is superseded by
this correction. Final freeze/gate results and independent clearance follow.


### Final corrected frozen gates

- `cargo fmt --check` passed: `/tmp/drudwyn-ticket14-final-fmt.log`.
- `cargo test --locked` passed all 41 Rust tests:
  `/tmp/drudwyn-ticket14-final-rust.log`.
- All 15 focused promotion/cleanup command and actual terminal tests passed in
  21.588s: `/tmp/drudwyn-ticket14-correction-focused-final.log`.
- Corrected Global pending-Finish replacement regression passed in 0.553s:
  `/tmp/drudwyn-ticket14-correction-global.log`. Corrected v2 compatibility
  passed: `/tmp/drudwyn-ticket14-correction-v2.log`.
- `bash tests/run.sh` exited 0 on the final corrected frozen code:
  `/tmp/drudwyn-ticket14-final-full.log`. Counts: batch UI1, launch15, recovery17,
  integration20, conflict17, verification10, promotion/cleanup15, Global11,
  status11, activity39, navigator13 (one existing optional Resurrect skip),
  independent clients29, settings10, plus all legacy shell, privacy, packaging
  and release-workflow gates. No full suites overlapped.
- `git diff --check` passed; generated `tests/__pycache__` removed before commit.

The independent reviewer reran the original active-shell reproduction after the
correction and confirmed both preview/removal refusals with checkout/window
preservation. Final delta review: Standards 0 unresolved severe, 1 nonblocking
maintenance judgement about duplicated shell classification; Spec 0 unresolved
findings. The fixed-SHA review receipt remains owned by the coordinating session
and reviewer after this atomic implementation commit.

All production/test changes preceded the final freeze. Only this completion
receipt followed the passing gates. Runtime evidence remains Linux/tmux 3.4;
other-platform/lsof behavior and unobservable external writers are not certified.
No push, deployment, branch deletion, main integration or live user fixture was
performed. Main remains `eaf24469290cbf77dd1d2a6176fbd54f7ace1868`.

## Independent Northstar crosscheck — 2026-10-01

Reviewed `6f69f7fd719d6bdd4b359f0b52890687b8dfabf5` through implementation
commit `f4328fc7100baca60948aed12db478bc63418682`, running Standards and Spec
sequentially under the implement-all override. Sources were CONTRIBUTING,
CONTEXT, the privacy boundary, ADR0002/0003/0005/0006/0007, this ticket and its
originating spec. All 14 implementation, test and documentation file hashes
match the reviewed corrected candidate; only this ticket's completion/gate
receipt changed after that freeze.

**Standards: clear — 0 severe, 1 deferred judgement call.** No hard documented
standard violation remains. Possible Duplicated Code: the shell executable-name
list is repeated in process classification and the final window-closure check.
A shared classification helper could reduce future maintenance drift. This is
a nonblocking heuristic, not a requirement to broaden this ticket.

**Spec: clear — 0 unresolved severe findings.** The independent public
reproduction found that an active Bash builtin wait loop was incorrectly treated
as idle, allowing both checkout removal and window closure. The correction now
refuses preview and removal while preserving that checkout and running window.
Reviewer evidence is `/tmp/drudwyn-review14-shell-{before,after}.log`; the unchanged
reproduction is `/tmp/drudwyn-review14-shell-repro.py`. Regression coverage includes
interactive/noninteractive shell builtins and invoking-shell background children
that change cwd. Explicitly stopped panes retain the mixed/coordinator/linked
window and session safety coverage. Independent promotion verification and
server-side pane/role/session-change checks also passed.

Final evidence inspected: formatting, all 41 Rust tests, and the complete
shell/Python suite passed in `/tmp/drudwyn-ticket14-final-{fmt,rust,full}.log`.
The full run includes all 15 promotion/cleanup cases in 21.849s and the updated
Global same-name replacement regression. The existing optional Resurrect skip
remains. Matching reviewed hashes required no redundant full-suite run.

Runtime evidence is Linux/tmux 3.4. Native macOS and its metadata-only lsof path
remain unverified; the portable process snapshot may conservatively refuse a
source-shell invocation because a completed probe still appears in its earlier
snapshot. This is a portability limitation to validate, not evidence of unsafe
removal. Unobservable outside writers and future external mutations remain
outside the documented observation guarantee.

This receipt changes only ticket14. Main remains
`eaf24469290cbf77dd1d2a6176fbd54f7ace1868`; no runtime edits, branch integration,
push or live user fixture were part of the independent review.
