# 06 — Launch a named worker with a complete task

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** ready

**What to build:** A launch form accepts a short name, editable branch, source
preview, agent, and usable task input, then creates and instructs the worker.

**Blocked by:** 01, 05.
**Priority:** P1. **Stories:** 1, 2, 3, 5, 6, 21, 22.

- [ ] Short name and worker branch are independent of task prose; detailed
  instructions do not generate an oversized branch/directory name.
- [ ] Edit and paste multiline instructions at supported terminal widths, or
  select a repository task-file reference; show full content/reference for review.
- [ ] Validate task-file availability in the worker checkout, including the
  case where a file exists only as uncommitted planning work in the source checkout.
- [ ] Launch from the displayed pinned commit, in the intended project context,
  while preserving the coordinator and other terminal's selection.
- [ ] Report creation and task transmission separately; delayed startup, failed
  paste/submission, and uncertain delivery permit deliberate recovery without
  silently resending or claiming that the agent accepted/completed the task.
- [ ] Bind delivery to the intended pane/process, and remove transient buffers
  on success/failure. Retain only a deliberately selected file reference, never
  task text in options, arguments, logs, or files.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.
