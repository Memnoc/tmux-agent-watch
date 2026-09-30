# 02 — Navigate independently in two terminals

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** done

**What to build:** Two clients can view the same project's windows and navigate
through Drudwyn without moving the other client's selection.

**Blocked by:** None — can start immediately.
**Priority:** P0 investigation/P1 delivery. **Stories:** 9, 15, 16, 21.

- [x] Establish a reproduction using two actual attached clients and record
  which part is normal shared-session behavior versus a Drudwyn targeting defect.
- [x] Exercise window/session navigators, existing Cockpit Open, and status
  navigation; target the requesting client and preserve the other selection.
- [x] Provide independently navigable views of shared project windows without
  changing files/branches or duplicating worker processes.
- [x] Resolve stable IDs and session membership; missing/ambiguous clients and
  vanished targets report errors instead of navigating an arbitrary client/window.
- [x] Linked session views deduplicate worker identity and do not cause
  application-created views to be confused with unrelated user sessions.
- [x] Repeated attach/switch/detach operations have a documented view lifecycle
  and do not destroy windows still used by another client.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.


## Implementation receipt — 2026-09-30

- Reproduction: two actual PTY-attached clients on a disposable tmux 3.4 server.
  Native `select-window` moves both clients because selection belongs to a tmux
  session. Before the fix, workspace navigator Enter also moved the other
  client's selection; its untargeted switch and session-global selection were
  the Drudwyn defect. The first behavioral run failed on that selection change.
- Shared routing now resolves an exact client and stable target membership.
  It creates a grouped view only when another client occupies the destination,
  reuses the view, and removes only an unused application view on switch/detach.
  Window and pane/PID identities remain shared. No Git command mutates state.
- Window/session navigator keys, Cockpit Open, installed popup keys, real status
  mouse input, and the attention shortcut were exercised. Status ranges retain
  window IDs instead of reusable indices. The sidebar uses the same route;
  native/legacy navigation remains available.
- Additional red→green cases: linked discovery initially counted one worker
  twice; implicit invocation from a linked pane initially guessed a client;
  actual popup execution exposed tmux 3.4's unexpanded popup environment format.
  Each regression now passes. Missing clients, ambiguous clients, vanished
  IDs, replaced window indices, and invalid IDs leave both clients unchanged.
- `python3 tests/independent_navigation_test.py`: 9 passed. Fixtures use isolated
  real Git and tmux, actual controlling PTYs, UI key/mouse input, and verify
  retained user-grouped sessions, repeated switch cleanup, detach cleanup,
  unique pane/PID sets, clean checkout/branch, and unique worker counts.
- `cargo fmt --check`: passed. `cargo test --locked`: 33 passed.
  Full `bash tests/run.sh`: passed, including 13 navigator tests (one existing
  optional real-Resurrect test skipped), 9 two-client tests, 10 settings tests,
  legacy integration, lifecycle, start failure, privacy, packaging, and releases.
  An earlier full run hit the existing ordinary-shell context startup race in
  the status fixture; its standalone rerun and final full run passed.
- Behavior and view lifetime documented in `docs/usage.md`. No spec edits or
  changes to main. Independent Northstar crosscheck is assigned by the
  coordinating session immediately after this atomic implementation commit;
  its receipt/follow-up findings belong to that review checkpoint.

Limit: independent views separate window selection, not pane layout or input
into shared worker processes. Native tmux retains its normal shared-session
semantics until clients use distinct views. No real user tmux server was touched.
