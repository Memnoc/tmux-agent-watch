# 26 — Guided workflow and second live-test polish

Status: implementation and validation in progress
Spec: docs/specs/2026-10-04-worktree-feedback-comparison.md
Baseline: 0187ad2

## Approved scope

- Submit the supplied task after the editor consumes the paste; never resend automatically.
- Plain language: Start a task, Start from, Merge into, Merge changes, Run checks.
- Focus-specific right-hand explanations with examples; F1 help and scrolling at narrow sizes.
- Explicit main/separate merge branch choice; preserve preview and confirmation.
- Consistent Rosé Pine default, padding, field groups, focus, Task border, action strips and icons with text.
- Keep worker/project and folder identity visible across branch choice, preview and result.
- Separate worker completion, review, merging, worker-reported checks and checks of merged code.
- Render check receipts on separate lines, readable relative times, technical identifiers in Details.
- Reopened merged work shows Already merged with relevant actions; no redundant merge action.

## Seams and evidence

Existing approved seams: CLI plus disposable Git/tmux, controlled fake agents,
actual keyboard events and rendered terminal buffers (originating spec Testing
Decisions). No live conversation capture or real automated agent task.

Red: delayed raw editor enables bracketed paste, waits 1.4s, and treats an Enter
bundled into its initial read as paste. Old transport left PROMPT PRESENT without
HASH. Linux now observes only the unread tty byte count, waits for consumption
plus 750ms quiet, then validates identity and submits once. Five-second timeout
leaves uncertain delivery; no replay. Non-Linux retains the prior wait and needs
separate validation. This reproduces a timing failure, not proof of the live Codex
root cause; the next live run must establish whether manual Enter is eliminated.

Red: verification UI rendered PassedCheck and all receipt fields on one line.
Green: semantic lines preserve check identity, result and readable timestamps.

Validation receipts and review will be recorded after the full run.
