# 09 — Supervise workers in a global Cockpit

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** ready

**What to build:** The approved global overview becomes a real terminal inventory
for dozens of workers, with consistent evidence and reliable inspection/opening.

**Blocked by:** 03, 07.
**Priority:** P1. **Stories:** 13, 14, 15, 16, 22.

- [ ] Discover at least 36 unique workers across four projects; deduplicate
  linked session windows and retain unassociated ordinary agents.
- [ ] Provide project/attention grouping, name/ref/session/project/agent search,
  project/state filters, global totals, matching counts, and explicit empty results.
- [ ] Expose coordinator access without counting shells as agents; a project
  inventory route can also reach non-worker windows hidden by bar overflow.
- [ ] Separate the current-window marker from row selection; inspection does
  not navigate or clear attention. Preserve selection by ID through refresh.
- [ ] Show complete selected details, including unknown batch/integration/check
  fields until evidence exists; do not invent data or show placeholder actions as working.
- [ ] Narrow layouts, long/identical names, redaction, icon modes, and keyboard
  scrolling retain safe selection and access to all details.
- [ ] Capture representative refresh costs and reuse per-refresh metadata;
  failure/staleness is visible and cannot retarget a pending action.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.
