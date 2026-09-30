---
status: accepted
date: 2026-09-30
proposed-by: Codex
approved-by: Memnoc
---

# ADR-0006: Use two informative status rows and persistent attention

## Context

[ADR-0004](0004-clustered-status-and-grouped-navigation.md) allocated one row
to clustered status and another to a separator, but the
[status feedback report](../intake/2026-09-30-status-feedback.md) found worktree
identity, activity, and attention difficult to distinguish.
Memnoc selected two informative rows and persistent badges for the redesign.

## Decision

Use both existing status rows for workspace identity, lifecycle context, and
attention feedback, retaining a distinct selected-window marker and readable
worktree identification: variant A places stable window tabs first, with selected
context and aggregate attention second, and a configurable visible-tab cap.
Attention persists in labelled badges and counts, including for workers hidden
by overflow; the initial design has no transient notifications, blinking, or
automatic focus changes.
Preserve ADR-0004's grouped workspace navigator, native tree fallback, opt-in
Nerd Font glyphs, and content-blind metadata boundary.

## Considered options

- **Two informative rows with persistent attention** — chosen to make workspace
  identity and actionable feedback readable without increasing status height.
- **One informative row plus a separator** — superseded because the row must
  fit navigation, identity, Git context, and attention in too little space.
- **Transient notifications for each new input/failure event** — deferred;
  persistent feedback is the selected starting point.
- **Always fill wide terminals with tabs** — not required; Memnoc selected a
  configurable cap because the 160-column view could be distracting.

## Consequences

Selection, linked-worktree identity, agent identity, lifecycle, and integration
state are separate facts. Color reinforces labels; it is not the only signal.
Process presence alone does not establish active task work, a completion
percentage, or an ETA. Visiting a worker does not resolve its attention state.

Memnoc selected variant A from the throwaway UI spike on branch
`spike/status-feedback-20260930`, with a configurable limit on visible tabs.
The selected window always remains visible and attention counts include hidden
workers. Width may further reduce the visible count; remaining windows use +N
and remain available in the navigator. Stable window order is preserved.

Visible-tab density must account for readable spacing: separate the selection
dot from its index, names from status badges, badge text from its edges, and
adjacent tabs from each other. Prefer reducing the number of visible tabs before
aggressively truncating names. The selected highlight covers the full tab.

The preview proposes four tabs by default, plus selected-only, three, six, and
automatic-fit choices. The final setting belongs in the existing options editor
and tmux configuration; its exact default and range are subject to preview
feedback. Simulated browser rendering does not establish live tmux behavior.
