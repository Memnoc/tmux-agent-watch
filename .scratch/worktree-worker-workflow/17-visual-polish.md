# 17 — Restore visual hierarchy and Cockpit branding

Status: done
Spec: docs/specs/2026-09-30-worktree-worker-workflow.md
Baseline: b326a0c

User feedback: screenshots 2026-10-01 at 14:07:27, 14:07:40 and 14:08:00.
The user requested status overflow/label fixes, removal of raw @ window IDs,
a more aesthetic Cockpit and restoration of its previous logo.

Acceptance:
- Separate status names from badge backgrounds with a measured gutter; preserve
  width limits and the selected tab, attention and exit evidence.
- Avoid repeating the selected agent's full name in the context row; bound refs.
- Hide raw window IDs in overview rows; retain stable IDs internally and in
  Cockpit details/explicit operation confirmations. Navigator uses window numbers.
- Restore the bundled Drudwyn hound (a compact version at narrow widths), with
  balanced header space and readable project/name/state hierarchy.
- Use short project names, preserve full paths in details, and disambiguate
  same-name repositories. Four workers in four repositories fit at 84 × 27.
- Keep complete navigator footer controls visible, wrapping between actions.
- Preserve state provenance, redaction, filters, and client-independent actions.

Red regressions: branded header absent and title adjacent to badge style;
separate footer regression proves Esc/Close clipped at 84 columns. Regression
suite in tests/screenshot_regression_test.py; controlled tmux fixtures only.

Validation:
- 42 Rust tests passed, including header freshness/overlap at 48, 64, 70, 84,
  100, 120 and 160 columns (`/tmp/drudwyn-polish-rust-final.log`).
- 8 screenshot regressions passed in debug and against the rebuilt release
  (`/tmp/drudwyn-polish-release-regressions.log`), including four distinct repo
  groups, long-parent duplicate names and agents sharing window numbers across
  sessions.
- 13 installed status tests passed (`/tmp/drudwyn-polish-status.log`): widths,
  themes, two clients, attention and exit receipts, gutters and context.
- 11 global Cockpit tests passed (`/tmp/drudwyn-polish-global-final.log`), including
  36 workers, narrow/full details, redaction, stale actions and identity guards.
- 12 navigator tests passed; one existing optional Resurrect skip
  (`/tmp/drudwyn-polish-navigator-final.log`).
- Release build, formatting and diff checks passed. Actual fixture cells were
  visually inspected at 84 and 48 columns with color enabled. The agent harness
  has NO_COLOR set, so the visual preview explicitly unsets it for its own child.
- Additional final-release filter/overlap checks:
  `/tmp/drudwyn-polish-release-filters.log`.

## Standards

Independent reviewer screenshot_standards: zero outstanding findings after
recheck. Centralized width-aware project labels, preserved provenance before
optional refs, and bounded header rows. Dawn retains a dark logo backing so the
bundled white hound remains visible; other themes use their own base.

## Spec

Independent reviewer screenshot_spec: zero outstanding findings after recheck.
Fixed clipped freshness at intermediate widths, disappearing duplicate-name
disambiguation, and missing agent session identity. IDs remain authoritative for
actions and available in details/confirmation, while overview rows stay readable.

Release binary rebuilt for the next popup invocation. Main unchanged at
`eaf24469290cbf77dd1d2a6176fbd54f7ace1868`; no merge or push. This was a focused
presentation follow-up; integration/cleanup implementation is unchanged.

