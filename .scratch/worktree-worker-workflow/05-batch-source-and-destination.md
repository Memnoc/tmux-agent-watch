# 05 — Choose the batch source and integration destination

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** ready

**What to build:** Batch setup pins a worker source commit and asks once whether
to integrate directly into the configured base or a named integration branch.

**Blocked by:** 03.
**Priority:** P1. **Stories:** 2, 3, 4, 21, 22.

- [ ] Preview the source ref and resolved commit, including configured-base,
  current-branch, and explicit locally available ref choices; do not fetch implicitly.
- [ ] Retain the pinned source for sibling launches despite navigation, target
  merges, or later source-ref movement; changing it requires deliberate selection.
- [ ] Show the base's actual configured name, or collect an integration-branch
  name and starting point; keep the selected destination visible for the live batch.
- [ ] Locate an appropriate destination checkout or offer a dedicated one;
  preserve the coordinator's original branch and never reset an existing branch.
- [ ] Invalid refs, name/path collisions, dirty source expectations, cancellation,
  and metadata loss have explicit outcomes; source and destination are distinct.
- [ ] Multiple batches for one repository retain independent associations;
  metadata is live-only and a missing association is unknown rather than guessed.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.
