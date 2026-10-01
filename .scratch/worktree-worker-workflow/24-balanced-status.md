# 24 — Approved three-region status A

Status: implemented, verified and applied
Baseline: 4d1df4d
Spec: docs/specs/2026-09-30-worktree-worker-workflow.md

Approved by Memnoc: “I think A is really good, let's go with that”.
Prototype: spike/three-region-status-20261001 at 6348923.

Acceptance:
- Balanced is the default layout: only current workspace at left (agent/shell
  icon, worktree/coordinator role, short name, activity), selected branch and
  tracked Git +/- physically centered, global NEED and total agents at right.
- Generous gaps; shorten names/branch before hiding total agents. Keep attention,
  stale/overlap evidence and selected identity truthful. Below 40 columns compact
  to existing Focus behavior. Never overflow terminal width.
- Two terminal rows: quiet full-width separator adjacent to terminal output,
  then information. Respect top/bottom status position. No third padding row.
- Rosé Pine default; existing theme/icon/redaction settings continue to apply.
- Current workspace opens its local workspace overview; NEED opens global
  attention; total agents opens global Cockpit. Clicks target requesting client.
- Preserve Focus/Tabs/Dense options and restoration when disabling HUD.
- Public CLI + installed private tmux seams agreed by the ticket-plan; use fake
  agents only. Full repository gates + independent crosscheck before commit.
- Rebuild and apply approved layout to live tmux after verification; no live
  worker mutation, no merge/push/deploy, main unchanged.

Implementation and evidence:
- Default setting and installer select Balanced; top/bottom place separator next
  to terminal output. Disabling HUD restores every prior status row and style.
- Agent icon selection reuses configured icon policy; shell icon is distinct.
  Rosé Pine foam provides center context colour; variants remain supported.
- Width allocation reserves complete right region if minimum Git context fits,
  then gives remaining center space to branch. Public regression reproduced the
  review finding at 64 columns with 12 agents and stale evidence before fixing it.
- Initial default/installed-render test red before implementation; final four
  public CLI/private-tmux status tests passed. Coverage includes clicks, two
  clients, 20–160 columns, long names, custom icons, worktree exits, overlap,
  stale evidence, redaction, all themes, top placement and original HUD restore.
- Actual tmux cells inspected at 48, 80 and 160 columns; HTML capture is
  `/tmp/drudwyn-status24-actual.html`. Only controlled fixture agents were used.
- Independent crosscheck against `4d1df4d`: Standards fixed missing help and
  shared icon policy; Spec fixed branch-before-agent-count compression. Both
  final reviews report no remaining findings.
- Logs: `/tmp/drudwyn-status24-red.log`, `...-reserve-red.log`,
  `...-balanced-final.log`, `...-rust.log`, `...-full.log`.

Completion:
- All repository gates passed: full `tests/run.sh` through packaging/release,
  then both assembled workflows rerun after updating their old Focus/default
  assertions to Balanced. No production code changed for that final test update.
- Rust library: 43 passed; new Balanced status checks: 4 passed; independent
  navigation: 30 passed; settings: 11 passed; assembled workflows: 2 passed.
  Existing optional tmux-resurrect test skipped; no new skips.
- Release binary rebuilt. Live tmux now uses `balanced`, Rosé Pine, two rows;
  existing top position retained, with separator below the information row.
  Verified installed status-format commands point to this repository.
- Final assembled log: `/tmp/drudwyn-status24-assembled-final.log`.
- Implementation stays on `work/worktree-worker-workflow`; main remains
  `eaf24469290cbf77dd1d2a6176fbd54f7ace1868`. No merge, push or deployment.
