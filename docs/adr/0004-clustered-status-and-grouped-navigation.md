---
status: accepted
date: 2026-09-02
proposed-by: Memnoc
approved-by: Memnoc
---

# ADR-0004: Cluster workspace status and group navigation

## Context

The persistent sidebar duplicated cockpit information, consumed terminal space,
and still did not expose every tmux window. A live tmux spike compared compact
tab treatments, agent grouping, contextual centre content, overflow signals,
Nerd Font glyphs, and a replacement for `choose-tree`.

## Decision

Disable the sidebar by default. Render a two-line tmux status area with ordinary
workspaces on the left, content-blind Git change context in the centre, managed
agent workspaces on the right, and a separator against terminal content. Use
lifecycle colour for agents and a fixed pointer for the selected agent. Nerd
Font glyphs are opt-in; the default remains dependency-free.

Bind `prefix+w` to a compact Ratatui navigator that groups all tmux windows into
ordinary workspaces and agents, orders agents by attention, supports filtering,
and jumps through tmux. Preserve native `choose-tree -Zw` on `prefix+C-w`.

## Consequences

Visual order no longer mirrors tmux window order, so window indices remain
visible and the navigator is the authoritative complete inventory. Status
surfaces show lifecycle and repository metadata but never prompt or response
content. Git line counts are live estimates: untracked files are counted
separately and binary changes contribute files but not lines.
