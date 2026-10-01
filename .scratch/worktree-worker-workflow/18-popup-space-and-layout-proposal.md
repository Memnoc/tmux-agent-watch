# 18 — More popup space and a Cockpit layout proposal

Status: done (size change and proposal; layout selection remains open)
Spec: docs/specs/2026-09-30-worktree-worker-workflow.md
Baseline: 012ea44

User feedback: screenshots 2026-10-01 at 14:42:40 and 14:43:02; enlarge Workspace
and Cockpit and propose an agent list closer to the approved overview mockup.

Implemented: both popups use 90% of the invoking client's width and 85% of its
height. The session chooser retains its existing dimensions. Client routing and
working directory behavior remain intact. Next popup invocation uses the change.

Validation: bash syntax and git diff checks passed; existing installed-popup
client-routing regression passed (1 test; /tmp/drudwyn-popup-size-check.log).

Proposal: throwaway branch spike/cockpit-layout-20261001, commit df0d33b,
worktree /tmp/drudwyn-cockpit-layout-spike. Run
`python3 scripts/status-feedback.spike.py` there, then open
http://127.0.0.1:8787/status-feedback.spike.html?view=overview&layout=A.

A is the recommended project table, B project cards, C attention lanes.
Floating arrows switch layouts; the example selector provides 4 or 36 workers.
All data/actions are simulated. Chromium screenshots inspected for all layouts,
plus A with 36 workers at wide and narrow widths. Prototype code stays outside
the implementation branch. No production redesign has been selected yet.

UNKNOWN means activity/ownership could not be established; RUNNING only proves
process presence. The proposal uses UNCONFIRMED and explains the evidence in
the selected-worker details. See ticket 19 for the newly reproduced hook failure.
