# 23 — Agent grouping and three-region status proposal

Status: grouping implemented and verified / status proposal awaiting approval
Spec: docs/specs/2026-09-30-worktree-worker-workflow.md
Baseline: 1130dd8

User requests:
- Submit a design for approval: current workspace only, start/middle/end regions
  with generous spaces; include a separator even if it costs another terminal row.
- Group agents together in Workspace so they are immediately easier to find.

Acceptance:
- Agents/workers occupy the top Workspace section, manual shells/editors below.
  Keep session context within each, stable window targets, kind/search filtering,
  existing actions and selection. Separate scroll areas keep agents visible when
  a manual shell is selected; keyboard order matches visual section order.
- Prototype three information hierarchies on the existing status-feedback route,
  Rosé Pine, one current-workspace identity only, anchored start/center/end and a
  real separator row. Optional bottom blank row makes the height choice concrete.
- Simulate all preview values/actions. Do not change the live status layout before
  user approval. Archive prototype separately and document its pointer.
- Use existing public CLI/private tmux rendering/action seams; full checks and
  independent review before committing implementation.

Implementation:
- Agent-first discovery ordering matches keyboard navigation and separate scrolling
  sections; session totals count only the current section's matching windows.
- A short section uses compact rows so selection remains visible at 48×20.
- Existing bot/manual-shell icons and stable action targets remain. Search and
  kind filtering preserve the selected window when it still matches, preventing
  an ordinal shift from selecting a different same-named project.

Design archive:
- Branch `spike/three-region-status-20261001`, commit `6348923`.
- Worktree `/tmp/drudwyn-three-region-spike`.
- Run `python3 scripts/status-feedback.spike.py`; open
  `http://127.0.0.1:8790/status-feedback.spike.html?variant=A`.
- Browser checked 75 variant/width/scenario combinations: no clipping, horizontal
  overflow or loss of global NEED; physical center alignment within one pixel.
  Variant switching, simulated inspection and optional padding controls passed.
- No live tmux status options were changed. Existing dense six-tab bar stays until
  approval of the proposed current-workspace layout.

Verification:
- Rust library: 43 passed; terminal rendered regressions: 14 passed.
- Mixed-group navigation crosses both directions at 120×32, 48×24 and 48×20;
  selected manual shell and agent section remain visible; filtering verified.
- Full run stopped when it picked up an in-progress version of the new regression
  using unsupported Home navigation. Corrected to existing Up/Down controls and
  reran the complete rendered suite; remaining full-run gates run separately.
- Navigation checks exposed that same-name selection regression; fixed identity
  preservation and reran all 30 independent-client navigation checks successfully.
- Independent crosscheck against `1130dd8`: Standards no findings; Spec found
  short-section selection visibility, fixed and re-reviewed with no findings.
- Logs: `/tmp/drudwyn-group23-full.log`, `...-screens-final.log`,
  `...-rust-final.log`, `...-tail.log`; proposal `/tmp/drudwyn-three-region-check.log`.

Completion:
- Every `tests/run.sh` gate passed across the initial full run, targeted reruns
  after the fixes, and remaining-gate runs. Status Tabs/Focus/Dense, lifecycle,
  activity, navigator, settings, privacy, packaging and release checks passed.
  One existing optional tmux-resurrect check skipped; no new skipped checks.
- Release rebuilt; both assembled workflows passed (direct base and integration
  branch/promotion). No live worker was used as a test fixture.
- Final independent Standards and Spec reviews: no remaining findings.
- Additional logs: `/tmp/drudwyn-group23-navigation-final.log` and
  `/tmp/drudwyn-group23-final-gates.log`.
- Main remains `eaf24469290cbf77dd1d2a6176fbd54f7ace1868`; no merge/push/deploy.
