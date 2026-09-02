---
status: accepted
date: 2026-09-01
proposed-by: Memnoc
approved-by: Memnoc
---

# ADR-0002: Rebuild v2 as a stateless Rust workspace supervisor

## Context

The Bash implementation proved the value of ambient agent status, linked-worktree workflows, and an action-oriented cockpit. It also coupled terminal rendering, input handling, tmux discovery, Git rules, agent classification, and mutations across a growing collection of scripts. Single-key menus cap lists at nine items, lifecycle surfaces assemble subtly different views, and content summaries are copied from hooks or terminal scrollback into tmux options.

The v2 design must retain the tmux-native workflow while establishing a strong privacy boundary for global distribution, including the EU/EEA. The project will not retain user content, operate a backend, or infer summaries, recommendations, productivity, or decisions.

## Decision

Rebuild v2 around one small Rust binary. A thin tmux entrypoint installs options, bindings, and hooks; the binary owns typed configuration, tmux/Git discovery, a unified `Workspace` model, deterministic lifecycle adapters, Ratatui rendering, and safe workspace commands.

The binary is short-lived and local-only. It runs no daemon, database, telemetry, analytics, crash uploader, update ping, remote API, history, or content cache. It never reads terminal scrollback or stores prompts, responses, permission details, or derived summaries. A task entered during workspace creation may be routed directly to the chosen third-party agent and is then discarded. Tmux may hold only live non-content operational metadata for the current session.

The cockpit becomes the primary action surface: a fleet overview ordered by
attention, with selection-based actions, filtering, scrolling, inline errors,
and Rose Pine themes and variants. The status bar is the default ambient
projection; the sidebar is opt-in compatibility. Direct shortcuts remain expert
paths. ADR-0004 records the grouped navigator added after the v2 boundary.

The executable exposes explicit subcommands for installation, scanning, the cockpit, sidebar/status rendering, and workspace start/review/jump/finish operations. Codex, Claude Code, OpenCode, and unknown agents map through adapters into a canonical lifecycle state machine. Git cleanliness and merge eligibility remain orthogonal workspace properties.

The rewrite proceeds alongside v1. The first milestone is read-only discovery, the unified model, browsing, and jumping. Mutating worktree actions follow after those seams are proven. v2 ships as a major-version boundary with prebuilt Linux and macOS artifacts for x86_64 and ARM64, plus source and `cargo install` paths.

## Considered options

- **Continue expanding Bash** — rejected because application state, navigation, rendering, and safety rules now need stronger types and testable seams.
- **Rust binary with Ratatui** — chosen for a single distributable artifact, structured terminal interaction, bounded dependencies, and testable rendering.
- **Background daemon and persistent registry** — rejected because tmux and Git already own live truth and persistence would violate the product boundary.
- **Browser or desktop application** — rejected because it would duplicate tmux navigation and introduce a separate runtime and data surface.
- **Content summaries from hooks or scrollback** — rejected because useful navigation does not justify retaining or duplicating agent conversation content.

## Consequences

The cockpit loses conversational summaries and instead shows fixed lifecycle labels; users jump to the source pane for details. Repository names, paths, branches, timestamps, and lifecycle metadata remain visible live but are never persisted by the project, and a redacted display mode is required for screen sharing.

Existing tmux option names and documented expert shortcuts remain compatible where sensible. Bash entrypoints may temporarily wrap binary commands, but there will be only one real domain implementation. The old Bash release remains available during migration.

Any proposal for model inference, content capture, telemetry, remote services, shared dashboards, persistent history, support-bundle upload, employee monitoring, or decisions about people requires a new compliance review and architectural decision before implementation.
