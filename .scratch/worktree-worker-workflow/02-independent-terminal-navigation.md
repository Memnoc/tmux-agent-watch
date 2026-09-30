# 02 — Navigate independently in two terminals

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** ready

**What to build:** Two clients can view the same project's windows and navigate
through Drudwyn without moving the other client's selection.

**Blocked by:** None — can start immediately.
**Priority:** P0 investigation/P1 delivery. **Stories:** 9, 15, 16, 21.

- [ ] Establish a reproduction using two actual attached clients and record
  which part is normal shared-session behavior versus a Drudwyn targeting defect.
- [ ] Exercise window/session navigators, existing Cockpit Open, and status
  navigation; target the requesting client and preserve the other selection.
- [ ] Provide independently navigable views of shared project windows without
  changing files/branches or duplicating worker processes.
- [ ] Resolve stable IDs and session membership; missing/ambiguous clients and
  vanished targets report errors instead of navigating an arbitrary client/window.
- [ ] Linked session views deduplicate worker identity and do not cause
  application-created views to be confused with unrelated user sessions.
- [ ] Repeated attach/switch/detach operations have a documented view lifecycle
  and do not destroy windows still used by another client.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.
