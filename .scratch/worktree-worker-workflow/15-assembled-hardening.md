# 15 — Harden the assembled workflow

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** ready

**What to build:** A reproducible acceptance run proves the integrated features
work together, followed by a real terminal visual review and release evidence.

**Blocked by:** 04, 08, 10, 14 (transitively all preceding tickets).
**Priority:** Release gate. **Stories:** 1–22.

- [ ] Run the complete supported Rust and shell/Python checks on the assembled
  revision, including preexisting lifecycle/status changes and compatibility paths.
- [ ] Create a session, launch three named workers from one commit, preserve
  coordinator identity, independently navigate two clients, and recover an exited
  worker through Drudwyn actions using controlled agents.
- [ ] Integrate independent commits with one shared-file conflict, hand off to
  the coordinator, Continue/Abort as applicable, and verify the assembled checkout.
- [ ] Exercise direct-to-base and integration-branch workflows including explicit
  promotion, failed verification from an untracked input, rerun, and guarded cleanup.
- [ ] Inspect actual terminal status/Cockpit layouts at all specified widths,
  icon modes, themes, redaction, 36-worker scale, and independent client actions.
- [ ] Record discovery/rendering costs and responsiveness with many Git checkouts;
  investigate regressions rather than borrowing the browser spike's performance.
- [ ] Confirm documentation/help/configuration, content-blind data flow, no implicit
  push/deployment, and crosscheck findings. Record limitations before calling it shipped.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.
