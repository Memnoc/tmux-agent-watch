# 22 — Approved Workspace and Sessions layouts

Status: done
Spec: docs/specs/2026-09-30-worktree-worker-workflow.md
Baseline: e5becdc
Reference: spike/workspace-session-20261001 at dd1130f

Memnoc approved both navigation proposals: “yes, I like them, proceed”.

Acceptance:
- Shared Rosé Pine masthead, quiet rules, tinted selected row and inspection
  sidebar matching the approved Cockpit language.
- Workspace table groups windows by session; role/name, tool, activity, branch
  yield columns responsively. Coordinator, worktree, agent and shell/editor
  identities remain clear. Search and agent/shell filters preserve stable targets.
- Sessions prioritizes names with window/agent counts and Current/Attached/
  Detached connection evidence. Selected details use observed data only.
- Both surfaces visibly expose New shell session using the existing safe creation
  command and name/directory form; cancel returns to the originating navigator.
- Existing rename, kill confirmation, save, coordinator and requesting-client
  navigation remain. Inspection never changes another terminal's selection.
- Narrow widths prioritize names; full selected details remain reachable. No
  orphan labels, invented activity, persistent state or secret-label leakage.
- Public CLI/private tmux rendering/action seams from the originating spec;
  fixture workers only. Full suite and independent crosscheck before commit.

Additional steering during implementation:
- Restore bot identity icons in Workspace, Sessions and Cockpit; distinguish
  manual shells/editors with a terminal icon. Preserve safe-font mode and existing
  configured icon overrides. A bot means agent presence/ownership, not working.
- Add a denser single-row bar with six visible tabs, readable label gutters and
  quiet separators/background boundary. Preserve selected-window inclusion,
  overflow navigation, global attention, redaction and requesting-client routes.
- Asked whether separator + blank bottom padding should consume three terminal
  rows; absent an answer, retain the user's earlier one-row preference. Dense is
  an explicit layout choice; existing Focus and two-row Tabs remain available.

## Review and validation

Test-first public-UI regressions failed on the missing selected-item panels;
the Dense installer regression failed before its layout was implemented.
Final Rust tests: 43 passed. Rendered regressions: 12 plus the added all-surface
safe/Nerd bot/manual-identity check (13 total). Existing lifecycle checks now
open Details after filtering, because provenance is no longer repeated in every
list row; both affected checks passed on re-execution, preserving attention.

Full suite /tmp/drudwyn-nav22-full.log passed through layout/status gates and
37 of 39 activity cases before those two stale presentation expectations.
/tmp/drudwyn-nav22-activity-fix.log records their successful rerun.
/tmp/drudwyn-nav22-tail.log completes navigator13 (one existing optional
Resurrect skip), independent navigation30, settings11, privacy, packaging,
release build and both assembled workflows. Both routes used two clients,
three workers, recovery, conflict Abort/Continue, verification and guarded cleanup.
Focus4, Tabs13, and Dense2 checks passed. Dense includes installed terminal
cells in safe/Nerd modes, real attention and window clicks, other-client
preservation, six tabs at160, selected-tab preservation as width shrinks,
redaction, and a20-column stale/overlap boundary. Logs:
/tmp/drudwyn-dense-green.log, /tmp/drudwyn-nav22-screens.log,
/tmp/drudwyn-nav22-icons.log, /tmp/drudwyn-nav22-new-shell.log.

Standards review: missing session observations initially used a manual-shell
icon; corrected to neutral ?. Recheck clear. Spec review: zero-width dense tabs
could still emit a separator; added bounded Focus fallback and regression.
Recheck clear. Full-detail scroll positions now clamp in stored view state so
scrolling back from the end responds immediately.

Inspected private production captures at160×42,84×27,48×24. Final Sessions image:
/tmp/drudwyn-sessions-approved.png. Bot/terminal font behavior is also checked
in real tmux, not inferred only from the browser capture.

Applied live presentation only: Rosé Pine, dense, six visible tabs, bot icon.
No live session/worker mutation. Bottom blank padding remains unimplemented:
the optional vertical-space question was unanswered, so the announced one-row
assumption preserves terminal height; tab gutters/dividers and a contrasting
surface distinguish the bar. Both full-padding and compact interpretations were
presented to the user rather than silently adding three rows.

Main remains eaf24469290cbf77dd1d2a6176fbd54f7ace1868. No push or merge.
Hook attribution remains ticket19; this UI work does not claim to fix it.
