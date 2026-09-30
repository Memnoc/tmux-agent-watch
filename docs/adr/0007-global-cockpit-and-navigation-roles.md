---
status: accepted
date: 2026-09-30
proposed-by: Codex
approved-by: Memnoc
---

# ADR-0007: Use Cockpit for global supervision and navigators for movement

## Context

The bounded status-bar design in [ADR-0006](0006-two-row-status-and-persistent-attention.md)
needs a complete overview when dozens of workers are active. The
[global overview preview](../intake/2026-09-30-global-worker-overview.md)
demonstrated 36 simulated workers across four projects. Memnoc approved the
preview and explicitly agreed with the proposed role split.

## Decision

- **Status bar:** immediate awareness through bounded local window context and
  clearly labelled global attention counts.
- **Workspace Cockpit:** global supervision through a searchable worker
  inventory, project and attention filters, full details, coordinator access,
  and lifecycle/integration actions.
- **Workspace navigator:** quick movement between all windows, including shells.
- **Session navigator:** session navigation and management, including New Session.

Global scope is the connected tmux server. Keep the stateless architecture;
this decision introduces no remote aggregation or persistent worker history.

Use consistent worktree identity and state labels across the bar and Cockpit.
Global attention totals include hidden and filtered workers, deduplicated by
window identity. Bar attention counts lead to the matching Cockpit view.
Keep inspecting a worker distinct from opening its window; neither action alone
clears attention. Activity, integration, and verification remain separate facts.

## Consequences

The bar can remain readable without hiding the existence of urgent work.
Cockpit provides the depth needed to supervise many workers, while the
navigators retain focused movement and session-management roles.

The approved browser preview establishes design direction, not production
behavior. Exact tmux bindings, responsive rendering, refresh behavior, and
performance verification belong in the implementation spec and tickets.
