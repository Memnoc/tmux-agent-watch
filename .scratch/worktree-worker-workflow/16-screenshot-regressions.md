# 16 — Fix the first live screenshot regressions

Status: done
Spec: docs/specs/2026-09-30-worktree-worker-workflow.md
Baseline: 70470a46f36976ebd32652833f4ae5b53b6526f1

The user supplied three screenshots from 2026-10-01 at 12:59:30,
12:59:50 and 13:00:01. Earlier workflow acceptance did not cover the
readability of an ordinary, unregistered tmux session at popup dimensions.

Acceptance:
- Four ordinary workers remain visible together at 84 columns × 27 rows.
- Full details and integration/recovery actions remain discoverable and usable.
- Known Git repositories group ordinary agents without inventing coordinator,
  batch, integration or verification state. Linked checkouts count together.
- The ambient attention total explicitly names attention rather than looking
  like a total of zero workers.
- Numeric session names in the navigator have a label and separating space.
- Preserve filtering, label redaction, activity evidence and client independence.

Reproduction: `python3 tests/screenshot_regression_test.py` uses private tmux
sockets, actual PTYs and verified sleep executables named codex. Initial run:
4 failures in 1.789s. Cockpit reproduced exactly two visible rows, 0 projects
for a known repository, GLOBAL 0 for attention, and an unlabelled session 7.

Confirmed causes: medium-width layout reserved 70% of the body for selected
details; grouping used registered project sessions only; the attention total
and navigator session suffix had ambiguous labels. These observations do not
establish a hook failure or missing activity evidence.

Validation:
- Rust: 41 passed (`/tmp/drudwyn-screenshot-rust-final.log`).
- Screenshot regressions: 4 passed in debug and 4 passed against the rebuilt
  release binary (`/tmp/drudwyn-screenshot-release-regression.log`).
- Global Cockpit: 11 passed, including the 36-worker fixture and narrow/redacted
  detail matrix (`/tmp/drudwyn-screenshot-global-final.log`).
- Status: 13 passed, including actual two-client cells at 48/64/80/120/160
  columns (`/tmp/drudwyn-screenshot-status-final.log`).
- Navigator: 12 passed, 1 existing optional Resurrect skip
  (`/tmp/drudwyn-screenshot-navigator-final.log`).
- Help script checks, shell syntax, formatting and diff whitespace passed.
- Release build succeeded. This follow-up used focused verification; the full
  integration/cleanup suite was not repeated because those modules are unchanged.

The final checks also cover repository-filter continuity through the Windows
shortcut, unassociated exclusion for known repositories, and preserved narrow
exit receipts. The attention label uses ATTN below 64 columns. Both filtering
issues were identified during independent review and fixed before completion.

## Standards

Independent reviewer screenshot_standards: no documented violations or
outstanding judgement calls. Reused the existing shortcut definitions to
resolve the initial low-priority duplication finding.

## Spec

Independent reviewer screenshot_spec: no outstanding findings on recheck.
Resolved repository grouping/filter inconsistencies while retaining explicit
coordinator/batch associations and label redaction.

The release binary is rebuilt for subsequent popup/status invocations. Reopen
existing popups to load it. Main remains
`eaf24469290cbf77dd1d2a6176fbd54f7ace1868`; no merge or push.

