# 25 — Live worktree test findings, 2026-10-04

Status: reproduced observations; fixes pending
Spec: docs/specs/2026-09-30-worktree-worker-workflow.md
Implementation under test: 5a2c1d1 (checkpoint da60b1f)

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
The live test is incomplete: no worker implementation or merge verified yet.
User screenshots are the supplied evidence; no live conversation capture used.
