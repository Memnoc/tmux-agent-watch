# 04 — Create a shell session from Drudwyn

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** ready

**What to build:** Users invoke New Session from a command or the session
navigator, enter a name/directory, and arrive in a shell.

**Blocked by:** 02.
**Priority:** P1. **Stories:** 9, 10, 21.

- [ ] Expose New Session alongside existing management actions and through the
  command interface; show its shortcut/help without colliding with save or kill.
- [ ] Collect name and directory with an invoking-workspace directory default;
  display the resulting named shell session in navigation.
- [ ] Switch only the requesting client and leave existing workers and the
  second client's selected window/session unchanged.
- [ ] Cancellation, duplicate/invalid names, invalid directories, missing client,
  and command errors preserve existing sessions and allow a corrected retry.
- [ ] Names/paths containing spaces or shell metacharacters are treated as data;
  creation does not implicitly create a worktree, change a branch, or start an agent.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.
