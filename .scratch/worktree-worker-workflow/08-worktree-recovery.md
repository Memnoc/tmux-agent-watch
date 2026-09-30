# 08 — Recover a worktree and deliberately restart its task

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** done

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
