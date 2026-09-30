# 10 — Ship status-bar A with density controls

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** ready

**What to build:** The accepted visual design renders in tmux with stable tabs,
selected context, persistent global attention, and functional Cockpit links.

**Blocked by:** 09.
**Priority:** P1. **Stories:** 12, 13, 15, 16, 22.

- [ ] Use both existing rows: local tabs above, selected context/global attention
  below; preserve the selected dot, distinct roles, and evidence-backed activity.
- [ ] Provide the reviewed default of four tabs and selected-only/three/six/Auto
  settings through the options editor and documented tmux configuration.
- [ ] Count every local window toward the cap, preserve selected visibility and
  stable order, and accurately report hidden windows without truncating padding away.
- [ ] Separate dot/index, name/badge, badge edges, and adjacent tabs; validate
  actual cell widths at 48, 64, 80, 120, and 160 columns and both icon modes.
- [ ] Global attention matches Cockpit's snapshot semantics regardless of tab
  cap; badge routes open matching global filters and overflow reaches all hidden
  project windows, including ordinary shells.
- [ ] Keyboard routes and mouse targets preserve client independence; redraw,
  resize, themes, malformed settings, and redaction do not create stale click targets.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.
