# 19 — Codex hook ownership with daemon/helper processes

Status: diagnosed; fix outstanding
Spec: docs/specs/2026-09-30-worktree-worker-workflow.md
Baseline: 012ea44

User reports repeated `Hook failed / hook exited with code 1` and asks what is
needed from them. No user action or permission change is needed for diagnosis.

Confirmed local evidence (2026-10-01):
- ~/.codex/hooks.json installs Drudwyn command hooks. PostToolUse calls
  scripts/codex-hook.sh userPromptSubmit, so a persistent failure recurs after
  tool calls. All configured lifecycle commands share the same adapter.
- The current execution environment supplies TMUX_PANE=%9. Read-only tmux
  metadata identifies that pane in AOC-TS, marked ambiguous.
- Process-name/parent/executable metadata shows a standalone Codex process,
  its Codex app-server-daemon child and further Codex children below that pane.
  These are counted separately by lifecycle::observe_hook_target.
- Executed the actual installed adapter through a PATH tmux guard allowing only
  show-option/display-message/list-panes/list-windows and rejecting all writes:
  `PATH=/tmp/drudwyn-hook-readonly:$PATH scripts/codex-hook.sh userPromptSubmit </dev/null`
  Result: exit 1, `tmux-drudwyn: Lifecycle event has no unique matching live agent
  in its originating pane`. No mutation was reached or allowed.

The proximate cause is confirmed: multiple recognized live process candidates
cause the hook to reject attribution, and reconciliation projects UNKNOWN.
This is an activity integration failure, not evidence of coding-task failure.
No hook configuration was disabled and no lifecycle event was forged on live
user panes. The local error-log search supplied no independent hook stderr;
the direct guarded reproduction is the error evidence.

Fix requirements:
- Model supported Codex launcher/server/helper arrangements using content-blind
  process evidence, without treating arbitrary nested independent agents as one.
- Establish which client/pane owns an event with a shared daemon; do not assume
  daemon-inherited TMUX_PANE always identifies the active terminal client.
- Prevent helper or unrelated-client events from marking the visible worker
  working/review/needs-input. Keep PID/birth and replacement guards.
- Retain conservative ambiguity for genuinely independent live agents and
  explain unavailable activity in the UI. Do not silently swallow every error.
- Build regressions with fake process families and private tmux sockets; never
  launch real agent services in tests. Validate the actual configured adapter.

This receipt records diagnosis only. Do not mark the hook issue fixed based on
popup changes, visual proposals, or a bare successful scan.
