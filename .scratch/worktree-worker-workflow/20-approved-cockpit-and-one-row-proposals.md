# 20 — Approved Cockpit A and coherent navigators

Status: done (modal implementation and three bar proposals; bar selection open)
Spec: docs/specs/2026-09-30-worktree-worker-workflow.md
Baseline: 5b5a92a

User approval (2026-10-01): implement the Cockpit exactly as proposal A and carry
its shape language to Workspace and Session modals. Propose single-row status
bars with compact information and Git line counts; do not pick one for the user.

Visual reference: spike/cockpit-layout-20261001 at df0d33b,
/tmp/drudwyn-cockpit-layout-spike/scripts/status-feedback.spike.html?view=overview&layout=A.

Acceptance:
- Compact bundled hound masthead, Rosé Pine palette, quiet separators, tinted
  selected rows, a project-grouped table and a narrow selected-worker sidebar.
- Worker/agent/activity columns stay aligned; branch/Git/integration columns
  appear when width permits. Preserve role, current and inspected identities.
- Unknown activity is explicitly unconfirmed. Process, hook, exit, integration
  and verification evidence keep their separate meanings.
- Details (d) retains full identities, receipts, coordinator recovery, paths and
  scroll access. Existing lifecycle actions remain available through ?.
- Workspace and Sessions share masthead, colors, rules, row selection and
  whitespace. Both retain navigation, filters and explicit destructive confirms.
- Cockpit/Workspace use 90% × 85%; Sessions uses 80% × 70%.
- Existing redaction, stable selection, client routing and stateless behavior
  remain intact. No live tmux fixture or real worker launch in automated tests.
- Three single-row bar alternatives on a separate throwaway branch, with width
  controls, Git +/- fixture values, global attention and URL-stable switching.
  Production status bar stays as installed until a variant is selected.

Testing: public CLI/private-tmux rendered surface seam. New approved-column
regression failed before implementation (WORKER header absent), then passed.
Existing tests inspecting full metadata now explicitly open Details. Terminal
captures wait for a stable completed frame, preventing reads during a repaint.
Browser preview checks cannot replace production-terminal acceptance.

Hook ownership remains tracked separately in ticket 19; this presentation
change does not claim to fix it.

## Review and evidence

Standards: independent layout_standards reviewer found no actionable findings.
Spec: layout_spec found two issues, both corrected and cleared on recheck:
session identities now have priority at narrow widths (24/34/38/48/84-cell
regression failed before the fix), and the overview restores WORK/RUN/UNCONFIRMED
global counts plus matching per-project need-you counts.

Validation logs:
- /tmp/drudwyn-layout-rust-final.log: 43 Rust tests.
- /tmp/drudwyn-layout-screens-final.log: 9 rendered screenshot regressions.
- /tmp/drudwyn-layout-full.log: complete suite through the status gate; one
  stale expectation for raw project IDs was corrected to the friendly label.
- /tmp/drudwyn-layout-status-final.log: all 13 status tests passed after that
  assertion update, including actual mouse/keyboard navigation and 36-worker
  two-client rendering. Local-session scope stays explicitly labelled session.
- /tmp/drudwyn-layout-tail.log: remaining lifecycle, activity, navigators, settings,
  privacy, packaging, release build and assembled-workflow gates.

Inspected fixture terminal captures for all three modals at 160×42, 84×27 and
48×24. Final Cockpit image: /tmp/drudwyn-cockpit-final.png. Narrow full-detail
access and private labels retain the original public-UI checks. No production
status-bar implementation changes were made.

Single-row proposals: spike/single-row-status-20261001 at 316f73d, in
/tmp/drudwyn-single-row-spike. Run `python3 scripts/status-feedback.spike.py`;
open http://127.0.0.1:8788/status-feedback.spike.html?view=bar&variant=A.
A quiet tabs, B selected workspace/Git, C project activity. Browser inspection
verified all three at 48/64/80/120/160 columns: one row, no overflow, Git/attention
visible; variant switcher and overview link worked. See
/tmp/drudwyn-proposal-check.log. Preview values/actions are simulated.

Final result: all required gates passed across the initial full-suite run and
its status/remainder rerun. Rust 43; rendered screenshot regressions 9; global
Cockpit 11; status 13; activity 39; navigator 13 (one existing optional Resurrect
skip); independent navigation 29; settings 10. Packaging/privacy/release gates
passed. Both rebuilt-release assembled workflows passed: direct-to-base and
integration-branch/promotion, each with two clients and three workers.

Release binary rebuilt from the final source. The next popup invocation uses
the new design. No main merge, push, live-worker test launch, or status-bar
redesign activation occurred. Main remains eaf24469290cbf77dd1d2a6176fbd54f7ace1868.
