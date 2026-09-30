# 03 — Identify and return to the coordinator

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** done

**What to build:** A project has a recognizable coordinator and named windows
with visible roles, with a reliable return action from worker navigation.

**Blocked by:** 02.
**Priority:** P1. **Stories:** 8, 15, 16, 22.

- [x] Explicitly associate a coordinator workspace with the project context;
  support both a shell and an agent without counting the shell as an active worker.
- [x] Distinguish linked-worktree worker, ordinary agent, coordinator, and shell
  in navigators and existing Cockpit details using text as well as optional icons.
- [x] Names derive from a deliberate short name/branch fallback; process title
  changes do not replace managed names, and later user renames are preserved.
- [x] Starting and switching among three workers preserves the coordinator
  window ID and checkout branch; a vanished coordinator offers explicit recovery.
- [x] Same-name projects/windows, external unassociated agents, and label
  redaction remain navigable by stable identity.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.

## Implementation receipt — 2026-09-30

- Added explicit `coordinator set --window ID [--session ID]` and `coordinator
  open [--session ID]` commands. Project identity is a stable tmux session ID;
  its explicit repository association uses Git's absolute common directory.
  New workers inherit that association only from matching project context.
  Coordinator shell and agent windows are both supported without checking out
  another branch or treating a coordinator shell as an active agent.
- Workspace navigator rows and Cockpit inventory/details show textual roles
  and stable IDs. `c` returns through the existing client-local routing layer
  from either navigator or Cockpit. External agents keep unknown associations;
  vanished or moved-out coordinators report unavailability and an explicit
  shell/reassociation recovery route. Replacement names/indices never retarget
  the old association. Project/common-directory associations remain live tmux
  metadata only; usage and privacy documentation describe the boundary.
- `workspace start --name` supplies a deliberate short name; the branch is the
  fallback. Initialization disables automatic and terminal-escape renaming and
  restores the requested name once, covering process output emitted before
  new-window returned. Scans never rewrite later user renames. Existing
  coordinator names are preserved. Role/ID fields precede optional navigator
  metadata so they remain visible at narrow widths and with label redaction.
- Red/green evidence at the approved seams: the first real two-client test
  failed on missing `coordinator`; three-worker creation failed on missing
  `--name`; actual UI keys initially lacked worker roles/return routing; a
  48-column terminal-buffer test failed because the role was clipped; a moved
  coordinator incorrectly rendered `c return`; immediate process rename escapes
  replaced the chosen worker name. Each regression is now green. Permanent
  coverage includes three real linked worktrees, two actual attached clients,
  shell and agent coordinators, common identity from a linked checkout,
  same-name repositories/windows, external unknown associations, redaction,
  later user renames, coordinator deletion/replacement/reassociation, and all
  three UI return routes.
- Final verification: `cargo fmt --check`, `git diff --check`, and
  `cargo test --locked` passed (35 Rust tests). Complete `bash tests/run.sh`
  passed on the final code, including 16 real-client tests, 13 navigator tests
  (one existing optional Resurrect skip), 10 settings tests, launcher failure,
  lifecycle, privacy, packaging, and release checks. One earlier rerun exited
  128 in the unchanged shell status-bar test without a diagnostic; its isolated
  traced rerun and subsequent complete runs passed. No production or test change
  was made to hide that transient result.
- Builder self-review covered standards, the ticket/spec, identity/membership,
  metadata privacy, and name initialization. Per the coordinating session's
  approved workflow, an independent Northstar crosscheck follows this atomic
  implementation commit; that review has not yet been claimed as complete.

Limits: association selects one coordinator per project session. Batch source
and destination choices, recovery automation/resume, global Cockpit expansion,
and the new status-bar layout remain their separate tickets. Losing tmux
metadata loses these associations; Git worktrees remain. No user tmux server
was touched; `main` remains `eaf2446`.
