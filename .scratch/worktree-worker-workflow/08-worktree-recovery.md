# 08 — Recover a worktree and deliberately restart its task

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** done — independently reviewed through `bd2617b`; Standards and Spec clear of blockers

**What to build:** Cockpit exposes surviving worktrees for known repositories
and allows reopening a shell or deliberately restarting an instructed worker.

**Blocked by:** 03, 06, 07.
**Priority:** P1. **Stories:** 7, 8, 11, 15, 22.

- [x] Enumerate Git worktrees for the selected repository and distinguish those
  without live windows from active workers; include missing coordinator recovery.
- [x] Reopen the existing branch/path without duplicating a worktree, resetting
  files, or overwriting a live worker; preserve dirty and untracked work.
- [x] Present Open shell and Restart with task/reference distinctly, explaining
  that a fresh agent does not restore a conversation. Offer resume only when supported.
- [x] Recover a live task reference where still available, or require explicit
  re-selection after metadata loss; never invent old prompts, checks, or exit history.
- [x] Report deleted directories, locked/prunable worktrees, missing agents,
  unavailable task files, and racing window creation without destructive repair.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.

## Implementation receipt — 2026-09-30

- Added `workspace recover-list --repo` and `workspace recover --repo --path`
  with distinct `--shell` and `--agent` modes. Git's NUL-delimited worktree
  inventory is scoped to the selected repository. Live windows, stopped retained
  panes, and surviving checkouts without terminals remain distinct; historical
  prompts, exit receipts and checks are not reconstructed.
- Cockpit `o Recover` offers `s Open shell`, `t Restart with task`, and
  `c Recover coordinator shell`, with selection, refresh, live-window navigation,
  and mutation-free cancellation. Restart explains fresh conversation semantics,
  supports multiline text or a task reference, and requires deliberate batch
  selection or an explicitly blank/unassociated choice. No unsupported resume
  action is offered. Live references can be reviewed and reused; metadata loss
  requires explicit task/reference selection.
- Recovery reuses shared launch initialization and the original pane/PID/checkout
  delivery guard. Existing checkouts never enter new-allocation cleanup. A
  checkout-inode lock serializes recovery against another recovery/delivery;
  the creation child inherits the guard so an interrupted parent cannot release
  an outstanding creation. External racing windows are retained and reported
  before any task send. Files, branches, commits and stopped panes are preserved.
- Shell recovery has no agent lifecycle/delivery binding. Dirty/untracked Git
  state is measured; primary coordinator checkouts are not labelled linked
  worktrees. Coordinator and retained-reference recovery use lossless known
  path metadata rather than globally unescaping tmux's presentation strings.
  Agent resolution is checked before launch; missing task files, unavailable
  agents, locked/prunable/deleted checkouts, live-window collisions and uncertain
  creation/send outcomes remain explicit errors without destructive repair.
- Red/green evidence: inventory first failed on the missing command; shell
  reopening and instructed restart failed on missing CLI options; Cockpit
  failed on its missing recovery entry; retained panes lacked a stopped label;
  shell recovery initially inherited Running metadata; literal-dollar coordinator
  recovery exposed escaped display paths; and dirty/primary recovery exposed
  new-allocation metadata assumptions. Each now passes through public commands
  or real terminal keys.
- Focused verification: 12 recovery integration scenarios passed with actual
  attached clients and disposable Git/tmux fixtures. Coverage includes tracked
  modifications, untracked files, literal paths/references, explicit retry,
  stopped-pane preservation/reference reuse, lost metadata, coordinator recovery,
  missing resources, concurrent and external window races, early agent exit,
  cancellation and redaction at 48/64/80/120/160 columns. Fake executable paths
  are asserted before any restart; no installed real agent was used. Synthetic
  terminal captures: `/tmp/drudwyn-ticket08-recovery-{48,64,80,120,160}.txt`.
- Final frozen-code verification: `cargo fmt --check`, `git diff --check`,
  `cargo test --locked` (41 tests), and complete `bash tests/run.sh` all passed.
  The complete run included 15 launch, 12 recovery, 36 activity, 29 real-client,
  13 navigator (one existing optional Resurrect skip), and 10 settings tests,
  plus all shell, lifecycle, privacy, packaging and release checks. Log:
  `/tmp/drudwyn-ticket08-final-suite.log`. An earlier run encountered the new
  dirty/primary assertions before the shared-launcher corrections; the final
  frozen run passed without failures.
- Builder self-review checked repository standards, the stateless/privacy
  boundary, shared-launcher assumptions, client targeting, reference validation,
  and failure retention. Independent crosscheck remains for the coordinating
  session; this receipt does not self-clear that review.

Limits: recovery opens a fresh process and does not restore conversations or
invent unavailable history. A selected explicit batch supplies current live
association, not proof that a surviving checkout originated at its pinned source.
Conflicting live references require reselection. Tests use Linux tmux and `/proc`
working-directory metadata; other platforms retain tmux's metadata fallback.
Global Cockpit redesign and integration remain later tickets.

The implementation stays on `work/worktree-worker-workflow`; `main` remains
`eaf24469290cbf77dd1d2a6176fbd54f7ace1868`. Independent review follows this commit.

## Independent-review corrections — 2026-09-30

- Reproduced both reported P2s before changing code. The paused coordinator
  recovery overwrote a live coordinator selected during launch, and Cockpit
  passed an unmanaged agent's escaped dollar-containing display cwd to Git.
  Original reproductions: `/tmp/drudwyn-review08-coordinator-race.py` and
  `/tmp/drudwyn-review08-literal-ui.py`.
- Coordinator recovery now snapshots missing ownership and compares it again
  under the existing socket-directory guard immediately before assignment.
  Manual coordinator selection uses the same guard. It is released during
  launch, so a deliberate intervening choice wins; the losing recovery reports
  an explicit conflict and retains its window/checkout without sending a task.
  Assignment receives an explicit guard, never nests lifecycle acquisition,
  and passes the guard to every mutation child. Recovery acquires checkout
  before socket; coordinator setters acquire only socket. Bounded contention,
  kernel cleanup and child-held lifetime avoid stale tmux wait-for locks.
- Recovery entry now revalidates the selected window/pane/process and resolves
  known encoded checkout identity or native cwd metadata. It never unescapes a
  presentation string or borrows another pane's directory after disappearance.
  The vanished-pane test also exposed a clipped short error at 120 columns:
  footer sizing now includes wrapped action hints, preserving the explicit
  stable-target error without changing inventory selection behavior.
- Focused verification passed all 17 recovery tests, retaining the original 12.
  Added manual-selection competition, simultaneous recoveries in distinct
  checkouts, a real Cockpit action for an unmanaged agent in a literal-dollar
  repository, mutation-child guard retention after parent death, and vanished
  selected-pane behavior. The orphan test proves a competing set receives the
  bounded retry error until the outstanding child finishes, then succeeds.
  Corrected-behavior reproofs of both original scenarios passed:
  `/tmp/drudwyn-ticket08-coordinator-reproof.py` and
  `/tmp/drudwyn-ticket08-literal-reproof.py`.
- Platform limits remain explicit: unmanaged literal paths were exercised with
  Linux `/proc/PID/cwd`. Other platforms require known checkout metadata or a
  directly resolvable tmux cwd; no blanket display unescaping was introduced,
  and non-Linux runtime behavior is not claimed as tested.
- Final frozen-code gates passed: `cargo fmt --check`, `git diff --check`,
  `cargo test --locked` (41 Rust tests), and complete `bash tests/run.sh`.
  The complete run passed 15 launch, 17 recovery, 36 activity, 29 real-client,
  13 navigator (one existing optional Resurrect skip), and 10 settings tests,
  plus all shell, lifecycle, privacy, packaging and release checks. Log:
  `/tmp/drudwyn-ticket08-corrections-final-suite.log`. No implementation changes
  followed the frozen run. Both original review scenarios and all five added
  correction scenarios passed; independent re-review is not self-cleared.

Independent re-review remains pending after this correction commit.

## Independent final crosscheck receipt — 2026-09-30

Reviewed `git diff 918f088...bd2617b`, including the correction diff
`6d6d387...bd2617b`, against this ticket, the originating specification and
implementation plan, CONTRIBUTING.md, CONTEXT.md, privacy documentation and
the applicable accepted ADRs. Standards and Spec were reviewed sequentially
under the implementation workflow. The independent reviewer session was reused
because of the harness thread limit; it reported both original P2 findings and
did not implement the ticket or its corrections.

Standards: no blocking violation or new material smell found. Existing checkout
recovery never enters allocation cleanup. The shared launcher retains the
original pane/process/checkout delivery binding for agents, while shells have
no agent delivery state. Path and task-reference encodings remain non-content
live metadata; no task contents, conversation recovery, durable registry or
filesystem crawl is introduced. Coordinator mutations explicitly receive the
existing guard, including inheritance by outstanding mutation children; the
checkout-before-socket order has no reverse coordinator acquisition.

Spec: no outstanding ticket-08 finding. Surviving checkouts, live windows and
retained stopped panes remain distinct. Open shell and instructed fresh restart
preserve existing dirty/untracked work and explain missing history. Explicit
batch choice and deliberate retained-reference reuse do not invent historical
association. Unavailable resources and competing creations produce retained,
inspectable failures. Both original P2s are corrected: an intervening coordinator
selection wins over a pending recovery, and Cockpit resolves a selected ordinary
agent's literal checkout without treating an escaped display label as a path.
A vanished selected pane produces an explicit error without borrowing another
repository. Later global Cockpit, status-bar and integration tickets were not
treated as missing recovery scope.

Independent frozen-code validation: all 17 recovery integration cases passed
in 24.243 seconds; log `/tmp/drudwyn-review08-final-recovery.log`. This includes
the original 12 cases plus distinct-checkout coordinator competition, manual
selection, orphaned mutation/bounded retry, literal-path UI recovery and vanished
selection. Both original reproductions were rerun with corrected assertions:
`/tmp/drudwyn-review08-coordinator-final.py` confirms an explicit conflict,
unchanged chosen coordinator and preserved new window/checkout;
`/tmp/drudwyn-review08-literal-final.py` confirms recovery inventory opens for
`repo $literal` despite tmux displaying its cwd with an escaped dollar. Fixtures
used disposable repositories, isolated tmux clients and verified fake agents.
No real installed agent or live user server was used.

The builder's fmt/Rust receipts and frozen full-suite log
`/tmp/drudwyn-ticket08-corrections-final-suite.log` were inspected; those gates
were not redundantly rerun or claimed as reviewer-owned full-suite evidence.
The optional Resurrect skip remains. `git diff --check` passed; the tree was clean
before this receipt and `main` remained
`eaf24469290cbf77dd1d2a6176fbd54f7ace1868`. Runtime evidence is Linux-only;
other platforms retain the documented known-identity/tmux-cwd fallback and were
not exercised. This commit changes only the ticket status and review receipt.
