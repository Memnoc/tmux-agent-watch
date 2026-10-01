# 21 — Rosé Pine polish, status B and navigation proposals

Status: done (implementation complete; navigation proposals await feedback)
Spec: docs/specs/2026-09-30-worktree-worker-workflow.md
Baseline: c7016ba

User (2026-10-01): selected B, requested another Cockpit polish pass and matching
Workspace/Session proposals, and corrected the default theme to Rosé Pine.

Acceptance:
- Install single-row B by default: selected role/name/activity, branch when space
  permits, tracked Git line counts against HEAD, clickable global NEED count.
- Keep stale/overlapping attention honest, redaction and stable client routing;
  retain two-row tabs as an explicit Appearance choice.
- Probe only the selected bound checkout, disable external diff/text conversion,
  isolate inherited Git namespace, report unavailable evidence as Git ?.
- Rosé Pine is the default throughout; explicit Moon/Dawn remain supported.
- Cockpit gives wide branch labels spare width rather than expanding worker
  names indefinitely; selected details use complete label/value pairs, explicit
  truncation and full detail access. Shared presentation retains readable contrast.
- Produce coordinated runnable browser proposals for Workspace and Sessions,
  with simulated data and no tmux mutations, archived outside production.
- Preserve main; validate public CLI/private tmux seams and independently review.

## Evidence

- Test-first focus failures: unsupported focus row/default two rows. During
  implementation, fixed tmux's one-row setting to `status on` (not `status 1`).
- /tmp/drudwyn-focus-green.log: 4 passing public-CLI/private-tmux tests, covering
  installed one-row output at 48/64/80/120/160, actual attention clicks, redaction,
  missing checkout, retained failed exit/review plus stale overlapping attention,
  and hostile Git namespace/external-diff isolation.
- Added a real Options-menu regression switching Focus → Tabs → Focus; theme
  preview now exercises the revised Rosé Pine default before choosing Dawn.
- /tmp/drudwyn-polish21-wide.log: wide branch visibility and complete
  Verification label/value regression passed.
- /tmp/drudwyn-polish21-rust.log: 43 Rust tests passed.
- /tmp/drudwyn-polish21-full.log: full suite through status tests; one old
  Options-menu row-index assertion needed updating for the new layout control.
  /tmp/drudwyn-polish21-tail.log reruns that case and all remaining gates.
  The earlier legacy palette assertion was updated from Moon to Rosé Pine.
- Inspected production fixture captures at 160×42, 84×27, 48×24 for all three
  modals. /tmp/drudwyn-cockpit-polish.png shows the revised Rosé Pine Cockpit.
- Preview archived on spike/workspace-session-20261001 at dd1130f;
  /tmp/drudwyn-navigation-spike, `python3 scripts/status-feedback.spike.py`.
  Workspace/Sessions share one route via ?view=workspace / ?view=sessions at
  localhost:8789. Selection, empty search and simulated shell form inspected
  at 1600/1000/700 pixels: no horizontal document overflow. Rosé Pine base
  confirmed as #191724. /tmp/drudwyn-navigation-check.log records the checks.
- Standards: one stale domain/usage documentation finding, corrected and cleared
  by independent recheck. Spec: no blocking findings. Both reviewers suggested
  extra Focus state/safety coverage; the added tests above pass.
- Rebuilt release and applied only presentation options/layout to live tmux:
  @drudwyn-theme rose-pine; @drudwyn-status-layout focus; status on; exactly
  one Drudwyn status-format row. No worker/session mutations were performed.

Hook ownership failure remains ticket 19, outside this change.

## Final result

All required checks passed across the full-suite run and focused continuations.
Rust 43; Cockpit global 11; rendered regressions 10; tabs status 13; Focus 4;
activity 39; navigator 13 (one existing optional Resurrect skip); independent
navigation 29; Settings 11. Privacy, packaging and release checks passed.
The assembled tests reached their final status assertion before finding a stale
two-row expectation; that assertion now verifies the installed Focus output.
/tmp/drudwyn-polish21-assembled.log: both rebuilt-release workflows passed,
covering direct integration and integration-branch promotion with two clients,
three workers, recovery, conflicts, verification and guarded cleanup.

Main remains eaf24469290cbf77dd1d2a6176fbd54f7ace1868. Nothing pushed or merged.
