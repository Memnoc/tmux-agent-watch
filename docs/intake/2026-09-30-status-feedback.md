# Status bar, attention, and progress representation

Date: 2026-09-30. Reporter and decision owner: Memnoc.
Status: Memnoc selected variant A with configurable visible-tab density. The
preview proposes a four-tab default. No live status-bar changes have been made.

## Source and requested outcome

Memnoc's follow-up to the [worktree workflow intake](2026-09-30-worktree-worker-workflow.md):

> We have added the red dot, great, but it's still really hard to tell if a worker is part of a tree, if it currently working, if it requires attention, etc.

> I think we need to re-design that part, the alert system and the progress system representation in the status bar does not satisfy the eye nor the functional feedback

The red dot is positively received. The requested redesign must improve both
appearance and the ability to distinguish location, worktree membership, activity,
and requests for attention. This expands W5 into a dedicated work area, W10,
within the core worker workflow; the earlier session and integration decisions
in [ADR-0005](../adr/0005-project-sessions-and-explicit-integration.md) remain accepted.

## Current implementation evidence

- [The status renderer](../../scripts/status-bar.sh) reads lifecycle, agent kind,
  branch, and repository metadata, but no linked-worktree identity. A displayed
  branch alone does not distinguish a linked worktree from the primary checkout.
- The selected agent gets a red dot regardless of lifecycle. State uses colors
  and, for attention states, text badges. A wide-bar working agent has no explicit
  working label; the compact bar does show `WORK`.
- Visible workers are chosen largely by window order and a width-dependent
  count limit. Overflow is a generic arrow; it does not expose hidden failure,
  input, or review counts. Below 80 columns only the selected context is shown.
- Attention labels reduce the available worker-name space, down to three
  characters in some layouts. That competes with recognizing the worker that
  needs action.
- [HUD installation](../../scripts/hud-install.sh) already allocates two rows:
  one for the clustered bar and one for a separator. Two informative rows could
  use the existing height, although that would revise ADR-0004's layout decision.
- [Lifecycle scanning](../../src/lifecycle.rs) reports `working` from process
  presence when no hook state is available. Its hook mappings carry explicit
  working/input/review events, but neither source establishes percentage complete.
- [Next attention](../../scripts/next-attention.sh) selects the oldest attention
  timestamp across sessions, preferring a different window. It does not rank
  failures above input requests or review, and grouped-session links will need
  deduplication and client-aware targeting.
- The [legacy sidebar](../../src/ambient.rs) already distinguishes linked
  worktrees with a diamond. There is existing visual vocabulary to evaluate,
  but it has not been adopted by the current clustered bar.

These are source findings, not a runtime reproduction or a visual acceptance
result. Existing pending red-dot and badge edits remain untouched.

## Proposed visual vocabulary

Each visual signal should answer one question and remain stable across widths.

| Signal | Proposed meaning | Presentation |
| --- | --- | --- |
| Selection | Which window am I in? | Keep the dot at a fixed position, with a subtle selected background |
| Role | Is this the coordinator, a worktree, or another workspace? | `COORD`, `WT`, or shell/agent identity; optional worktree glyph with a text fallback |
| Agent identity | Which agent is running? | Existing agent symbol where space permits; never substitute it for worktree identity |
| Activity | What lifecycle state is reported? | Short text badge; color reinforces the text |
| Attention | What needs me, including hidden workers? | Reserved summary with separate failure, input, and review counts |
| Integration | Where will this work go, and has it been integrated? | Selected-worker context; distinct from lifecycle and verification |

Suggested Rosé Pine treatments: quiet blue for working, gold filled badges for
input, red filled badges for failure, and teal for review. The red selection dot
remains a small location marker; failure also has a prominent shape and label,
so the two do not rely on red alone. Avoid blinking and automatic focus changes
as the starting proposal. These are design recommendations, not settled choices.

## Proposed layout

Use two informative rows in the space currently occupied by bar plus separator.
Worker order stays stable; urgency changes badges and the attention summary,
not the position of every window.

Illustrative wireframe, not a rendered tmux result:

```text
site  [0 COORD]  ●[2 WT nav WORK]  [3 WT header INPUT]  [+1]
nav · work/nav → main    NEEDS YOU: 1 input · 1 review    prefix+a
```

At narrow widths, retain selected-worker identity, worktree marker, lifecycle,
and attention counts. Collapse Git statistics, other names, and decorative
symbols first. Width budgeting must use actual terminal cell widths; this
wireframe is not proof that every translation, glyph, or name fits.

Proposed summary scope is the current project, with a separately labelled
count for other projects needing attention when applicable. Counts should refer
to unique workers/windows rather than links to the same window across grouped
sessions. Exact scope is a design detail still to settle.

## Truthful activity and progress

Lifecycle, integration, and verification must remain separate facts:

- **Starting:** launch is in progress; task acceptance has not been established.
- **Running:** process presence is known, but task activity is unconfirmed.
- **Working:** an explicit lifecycle event reports activity; this is not a
  heartbeat guarantee or an estimate of completion.
- **Input:** an explicit event indicates user attention is required.
- **Review:** an agent event or handoff calls for review. It does not establish
  that all acceptance criteria passed.
- **Stopped / failed / unknown:** distinguish an observed exit from an explicit
  failure and from missing evidence. Do not invent success or failure after a
  window vanishes.
- **Integrating / conflict / integrated:** report known Git operation/result
  state for the chosen destination, independently of whether the agent runs.
- **Verified:** only report checks actually run against the relevant assembled
  revision; otherwise show verification as unknown or not performed.

Durations, if shown, mean time since the reported state transition, not remaining
time. Do not derive percentages, ETA, or completed-task counts from elapsed time,
process presence, conversation content, or a turn-ending hook. No persistent
progress history or prompt capture is introduced.

The first implementation must distinguish already available evidence from
states that require new launch/integration events. A visual mock must identify
simulated states rather than implying those integrations already exist.

## Proposed attention behavior

- Keep input/failure/review signals visible until their underlying state
  changes. Merely visiting a window does not mean its problem is resolved.
- Keep aggregate attention visible even if the relevant worker is hidden by
  width limits or the selected workspace is an ordinary shell.
- Offer a deterministic attention route: failure, then input, then review;
  oldest first within each class. Display what the action will open and respect
  the requesting terminal's independent view.
- Consider a brief notification on a new input/failure transition, separately
  from persistent badges. Deduplicate transitions so polling does not repeatedly
  announce the same state. Whether notifications are enabled is a user choice.

## First design questions

1. **Layout:** use the existing two rows for navigation plus status/attention
   (recommended), or retain one information row and its separator?
2. **Notification behavior:** persistent badges and attention counts only
   (recommended starting point), or also a brief notice for each new input/failure
   transition? Neither proposal automatically switches the user's window.

### Decision and preview

Memnoc answered: “two is better” and “let's do as you say, let's see how it
looks.” [ADR-0006](../adr/0006-two-row-status-and-persistent-attention.md)
records two informative rows with persistent badges, without transient notices.

A standalone browser preview is isolated on branch
`spike/status-feedback-20260930`, in `scripts/status-feedback.spike.html`.
Run `python3 scripts/status-feedback.spike.py` from that worktree and open
`http://127.0.0.1:8786/status-feedback.spike.html?variant=A`.
The current review worktree is `/tmp/drudwyn-status-feedback-spike`.

- **A — Workspace tabs:** stable navigation first; selected context and
  aggregate attention on the second row. This is closest to the discussed design.
- **B — Focus first:** selected-worker context and attention first; navigation
  second.
- **C — Attention first:** the most urgent worker and aggregate attention first;
  stable navigation second.

The bottom switcher and left/right arrow keys select variants; the URL preserves
the variant. Controls adjust width, selected window, simulated worker state, and
theme. Simultaneous 64/48-column examples expose overflow behavior. All data is
simulated and all interactions are local to the page; this is not a browser UI
proposal for the product or a live agent integration.

Browser inspection verified the initial rendered rows and exercised selection,
state changes, variant switching, and theme switching. Selecting an attention
worker preserved its alert; changing its state updated the aggregate count.
Memnoc accepted A and requested customization of visible tab counts because the
160-column view could be distracting. Real tmux rendering, fonts, input,
and metadata discovery still require implementation and acceptance coverage.

### Visible-tab customization

The preview adds **Maximum visible tabs**: selected-only (1), 3, 4, 6, and Auto.
Four is the proposed default; Auto retains the previous fill-available-space
behavior. The cap counts coordinator, shell, and worker tabs together, excluding
the +N overflow indicator. The selected window is always included. Narrower
terminals may show fewer tabs than the cap; widening does not exceed it.

Failure, input, and review counts include all workers even when their individual
tabs are hidden. Normal window order stays stable rather than reordering tabs
on every lifecycle event. The full navigator remains the route to all windows.

Preview link: `http://127.0.0.1:8786/status-feedback.spike.html?variant=A&width=160&tabs=4`.
The tab preference is preserved in the preview URL. Production configuration
should expose the same control in the existing options editor (`prefix + O`)
and a documented tmux option; neither is implemented by this browser spike.

### Screenshot review: spacing and truncation

Memnoc supplied screenshots dated 2026-09-30 at 12:10:23, 12:10:35, and 12:10:43
showing the red dot touching the window number, `api` touching `FAIL`, and
`projects` touching `REVIEW`. The tab boundaries also lacked separation, and
narrow layouts shortened names aggressively even before considering fewer tabs.

The preview now gives the selection dot a separate cell from the number,
leaves two plain cells between names and status badges, pads each badge's text,
and separates adjacent tabs. The selected background covers the full tab.
All gaps count toward the width budget. Prefer fewer visible tabs over reducing
names to a few characters; reserve more name space for the selected window and
use an ellipsis when truncation is still necessary. This refines variant A and
retains the configurable cap and aggregate attention counts.

## Verification agenda

The [global overview extension](2026-09-30-global-worker-overview.md) now
demonstrates 36 workers across four projects on the same preview route using
`?view=overview`. The bar remains bounded; its attention counts explicitly cover
all projects and open matching Cockpit filters. +N opens the current project's
inventory. Memnoc approved the preview and surface role split, recorded in
[ADR-0007](../adr/0007-global-cockpit-and-navigation-roles.md). This remains a
browser prototype, not a live tmux feature.

Preview the same scenarios at 48, 64, 80, 120, and 160 columns: active and inactive
worktree workers, an ordinary agent, coordinator and shell windows, mixed input /
failure / review states, and hidden urgent workers. Cover safe and Nerd Font
modes, theme variants, long or identical names, and label redaction.

Check that process-only detection never claims known task progress; visiting an
alert does not falsely clear it; notifications do not repeat on each scan; hidden
attention remains visible; and navigation from either of two clients leaves the
other client's selection unchanged. Reuse the existing status, lifecycle, and
tmux integration seams once the design is agreed.

ADR-0006 supersedes ADR-0004's separator-row layout, retains its navigation
decisions, and records variant A plus configurable density as the chosen design.
Rewrite the production rendering deliberately rather than shipping spike code.
