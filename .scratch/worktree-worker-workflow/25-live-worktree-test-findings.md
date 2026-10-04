# 25 — Live worktree test findings, 2026-10-04

Status: reproduced observations; fixes pending
Spec: docs/specs/2026-09-30-worktree-worker-workflow.md
Implementation under test: 5a2c1d1 (checkpoint da60b1f)

## Consolidated feedback checklist

This checklist gathers the user's feedback from the October 4 hands-on run.
Suggested treatments below are follow-up proposals, not completed fixes.

- [ ] **Moving between testing and the coordinator is cumbersome.** The user
  must return to this chat to report results. Navigation worked; returning to
  chat must not be interpreted as selecting the wrong test session. Make the
  coordinator easy to identify and return to while preserving worker context.
- [ ] **Secondary popups look bland.** Bring batch setup, launch, recovery and
  integration into the approved Cockpit's Rosé Pine (not Moon) visual language.
  Improve hierarchy, spacing, focus and action styling.
- [ ] **Form editing and focus are unclear.** Short name, branch and multiline
  task navigation need obvious controls. The global Enter-newline hint is
  misleading outside Task. Destination entry also felt copy/paste-only; this
  is user-reported friction, not a reproduced keyboard-input failure.
- [ ] **Expose destination choices.** Offer selectable existing branches and
  checkouts, showing branch and directory together, instead of requiring a
  memorized branch name or batch ID. Explain branch-vs-path mistakes clearly.
- [ ] **Make Preview, Back and Close discoverable.** Enter retrying an invalid
  preview and nested Escape behavior felt like a trap. Cancellation failure
  itself has not been independently reproduced.
- [ ] **Repair activity and attention signals.** A worker that had committed
  its result still displayed process-derived RUNNING without a completion or
  review signal. Repeated Hook failed messages also remain unresolved (see
  ticket 19). Their causal relationship to this worker's state is unproven.
  Distinguish a live process from active work, input needed and review readiness.
- [ ] **Make progress and completion unmistakable.** Launch, recovery and
  integration need clear action/result feedback. After integration the screen
  still looked like a preview with an old destination commit. Show the actual
  result prominently and provide visible next actions, keeping integration
  and destination verification separate.

Additional observed failures and continuity questions:

- [ ] Runtime files disappeared when the development checkout returned to main.
  Stable preview loading mitigates exit 127; durable installation remains open.
- [ ] Recovery opened Codex but did not visibly deliver the supplied task,
  despite a sent receipt. Manual pasting was required. Startup readiness is a
  hypothesis, not a confirmed root cause.
- [ ] Launch presented an ERROR alongside successful creation/task transmission;
  clarify transport success, unknown acceptance and actual failure.
- [ ] Review recovery continuity: window name changed, launch used the requesting
  chat session, and blank batch association required a fresh destination choice.
  These observations are not all established defects or promises to add state.

## Current outcome and coverage

One real worker completed and was integrated through the UI after manual task
submission. The worktree survived the updater exit; recovery reopened the correct
checkout. Playground main reached adb161d, remained clean, and both tests passed
when checked independently. These checks do not register a UI verification receipt.

The second task, simultaneous workers, conflict handoff, integration-branch
promotion, verification through the UI and cleanup were not exercised manually.
The automated scenarios below are separate evidence using controlled agents.

## Runtime checkout dependency — mitigated

Switching the development checkout to main removed navigation-popup.sh while
live tmux bindings still referenced it. Direct invocation reproduced exit127.
A stable preview checkout now lives at /home/memnoc/Code/tmux-drudwyn-preview;
the live plugin bindings, watcher and sole ~/.tmux.conf Drudwyn loader use it.
Development checkout remains on main. Durable installation should avoid depending
on files disappearing during a development branch switch.

## Task handoff after update and recovery — open

User launched greetings from the disposable project
/tmp/drudwyn-worktree-playground-20261004 using the committed task01 instructions.
Codex displayed a successful update and requested restart. Original window @29
exited0; work/greetings remained at ce51f1e with no changes. Screenshot084011
shows a red ERROR message despite creation/task-send receipt; screenshot084038
shows the updater output. Exit0 did not establish task completion.

User selected Restart with task, pasted the instruction again, and launched.
Screenshot084720 shows fresh Codex0.160.0 in the correct worktree with an empty
prompt. Recovered window @30 is live; @drudwyn_delivery=sent, but Git remains
clean at ce51f1e. Thus transmission is recorded without observed task acceptance.
No automatic replay was performed during diagnosis.

Suspected readiness race: workspace::send_started waits for a matching process
name, then transmit_task pastes and submits. Process presence is explicitly not
input readiness. The real cause still needs a controlled delayed-startup/input
regression; do not claim diagnosis complete from the screenshot alone. Preserve
content-blind operation, no prompt persistence, stable process identity and no
unconditional replay that could duplicate an accepted task. Test through the
real launch/recovery adapter seam, using controlled agents for automation.

Recovery also created work/greetings in the requesting chat session7 rather
than drudwyn-test, and used the branch as window name instead of greetings.
Record this as observed behavior for UX review, not a proven association defect;
the user had returned to the chat to communicate. Blank recovery batch is an
explicit unassociated option today; inspect binding before later integration.

## Form usability — requested follow-up

User requests Rosé Pine styling for bland batch/launch/recovery popups: clearer
headings, active-field highlights, spacing and a distinct review summary.
User could not move from short name to Task with Enter/arrows. Tab currently
cycles Short name -> Branch -> Task; Enter newline applies only in Task.
Make this field-specific and obvious. Improve the launch receipt wording/tone
so task-sent/acceptance-unknown is not presented as generic ERROR success/failure.

## Test status

Automated assembled direct-base and integration-branch scenarios passed from
the saved branch on2026-10-04 (2tests,25.121s), with controlled agents. This does
not establish real Codex input acceptance; the manual test found that gap.
At the initial checkpoint, implementation and integration were still pending.
Both were subsequently verified; see Current outcome and coverage above.
User screenshots are the supplied evidence; no live conversation capture used.

## Integration destination form — live feedback, 08:59

After manual task submission, worker commit adb161d added greetings.py and two
passing unittest cases. Worktree was clean; playground main remained ce51f1e.
Process-derived RUNNING persisted, with no observed review/completion signal.

Recovered worker integration required choosing the destination again. User
reported difficulty entering a destination without copy/paste and leaving the
menu. Screenshot085922 shows the checkout path entered in Destination branch,
focus on Batch ID, and the generic exactly-one-checkout error. Enter retries
preview; code handles Escape by returning to Cockpit. A keyboard cancellation
failure has not been independently reproduced.

Required UX follow-up: list eligible existing destination branches/checkouts
with branch+path shown together; make explicit selection possible without typing
or pasting identifiers. Retain fresh preview/confirmation and stale-target guards.
Text entry, where retained, needs visible cursor/focus, branch-vs-path validation
and a useful error example. Distinguish Preview, Back and Close; make nested Esc
behavior obvious. Do not dismiss the discoverability problem as user error.

## Integration completed, but feedback was missed — 09:01

User pressed y on reviewed source adb161d into the playground main. Screenshot
090155 shows a subtle Integrated line while the title remains INTEGRATE PREVIEW
and the old plan, destination commit and review token still dominate the screen.
User could not tell whether any action occurred and reports that progression and
completion throughout the workflow are too quiet.

Independent verification after the action: playground main HEAD is adb161d,
source is its ancestor, working tree is clean, and both greeting unit tests pass
in the destination. This was the real UI-triggered fast-forward; no merge command
was run by the supervising agent. These independently run tests do not create a
Drudwyn verification receipt, so the UI's not-verified label remains truthful.
Development repository main remains eaf24469290cbf77dd1d2a6176fbd54f7ace1868.

Required UX: visibly transition from Review to Integrating to Integrated (or
Failed/Conflict), with a prominent result title and semantic colour plus text.
After success, show actual destination/head and changed-file summary, move the
old preview/token to details, and expose next actions (open destination, verify,
return). Keep worker completion, successful merge and destination verification
separate. Never invent completion percentages or imply that merge proves tests.
The same clear in-progress/result feedback is needed for launch and recovery.

Manual test checkpoint: one real worker created, interrupted by updater,
recovered with failed automatic task handoff, implemented after manual delivery,
and integrated successfully through Cockpit. Destination checks passed separately.
Second prepared task not launched; live multi-worker integration and cleanup not
yet exercised in this hands-on run. Automation's broader coverage remains separate.

## Implementation pass — 2026-10-04

Baseline: f530145. Memnoc authorized the entire functional and UX feedback set.
Implementation and verification receipt is being completed on the existing
feature branch; main is not the integration destination for this work.

Public seams remain the specification's CLI, real isolated Git/tmux, controlled
process families, and actual keyboard/rendered UI. New regressions reproduced:

- A Codex-named process exists 1.5 seconds before initializing terminal input;
  old delivery lost the task. Waiting for noncanonical/no-echo editor mode passes.
- A packaged daemon executable below the visible Codex terminal caused false
  ambiguity. Executable-location classification removes only that known backend.
- A hook inherited another client's TMUX_PANE; the old adapter marked the wrong
  worker. Routing by unique session cwd with process-lifetime revalidation passes.
  Ambiguous same-cwd clients are rejected; no payload is inspected.
- The destination screen offered no selectable main checkout. The picker now
  exposes branch/path and still requires preview plus explicit apply.
- Integration retained the preview presentation after success. The new result
  shows the actual destination commit and offers verification of that checkout.
- An open Cockpit kept stale lifecycle state. Passive overview refresh now shows
  attention without manual reload; active forms and review targets stay frozen.
- Coordinator/worker toggling affects only its requesting client.

Hook working-directory semantics were checked against the official
[Codex hook contract](https://learn.chatgpt.com/docs/hooks) on 2026-10-04: command
hooks execute in their session cwd. The adapter emits the neutral JSON result
required by Stop. Linux executable-location evidence identifies packaged Codex
backends; unfamiliar layouts still require explicit diagnosis rather than folding
all nested agents into one. This pass has not run real Codex services in tests.

All action screens use Rosé Pine hierarchy, focus colour, spacing and keycaps.
Recovery preserves validated surviving window/project identity and preselects the
retained batch. Startup/recovery show progress, successful transmission is no
longer a generic ERROR, and missing activity signals are explicitly explained.
A complete local runtime snapshot survives development checkout removal, with
atomic selection and retained prior snapshots.
