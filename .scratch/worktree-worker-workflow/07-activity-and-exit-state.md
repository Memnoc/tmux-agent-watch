# 07 — Show evidence-backed activity and exit state

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** ready

**What to build:** Users can distinguish a running process, reported task
activity, attention requests, and an agent that exited, across current surfaces.

**Blocked by:** 01.
**Priority:** P1. **Stories:** 6, 7, 13, 16, 22.

- [ ] A process without a supporting lifecycle event displays Running, not Working.
  Show evidence source and unknown state where attribution is unavailable.
- [ ] Supported events produce Starting/Working/Needs input/Review/Failed
  consistently; Review does not imply passing checks or integration readiness.
- [ ] Preserve inspectable exit code/time when available in live metadata;
  zero exit is distinct from task completion, and lost history remains unknown.
- [ ] Attention survives inspection and unrelated scans; newer authoritative
  evidence supersedes it, and a replacement process cannot inherit stale evidence.
- [ ] Split-pane/active-pane changes do not reclassify the wrong process or route
  events to a different worker; unknown multi-agent ownership remains explicit.
- [ ] Existing pending lifecycle work is reviewed and covered by behavior tests;
  fixed metadata replaces content inspection and no inferred percent/ETA is added.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.
