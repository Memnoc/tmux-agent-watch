# October 4 worktree feedback: repeat comparison

The first live run used `/tmp/drudwyn-worktree-playground-20261004`, starting at
`ce51f1e`. Its completed greeting worker and integrated main are preserved.
The repeat fixture is `/tmp/drudwyn-worktree-comparison-20261004`, restored to
that exact starting commit with the same two task files and no remote. No real
agent has been launched in the repeat fixture by the automated checks.

## Repeat the original route

1. Open Sessions (`prefix + s`) and select `drudwyn-compare`, window `plan`.
   Its coordinator is registered; `prefix + G` toggles between this coordinator
   and the last worker for the requesting terminal only.
2. Open New worker (`prefix + W`). In batch setup, retain source `base` and
   a blank integration branch for direct integration into main. Review the
   actual source/destination before confirming. Main must show `ce51f1e`.
3. Short name: `greetings`. Enter/Down advances to Branch; then Task. Paste the
   contents of `tasks/01-greeting.md` into Task, including its line breaks.
   Alternatively F5 selects the committed task file; record which route is used.
   F6 starts the worker. Check the creation notice and open the worker.
4. Observe actual task acceptance and implementation. A sent receipt alone is
   not acceptance. New default Codex workers use `--no-daemon` so their hooks can
   prove terminal ownership. Check Working, input-needed when applicable, then
   Review when the agent reports completion; an idle live process alone is Run.
5. Return to Cockpit (`prefix + P`). It refreshes every three seconds when no
   form/detail view is open. Open Integrate for greetings. If no live batch
   remains, select the displayed `main` + comparison directory; no identifier
   needs typing. Enter previews; y applies only the reviewed state.
6. Check the **INTEGRATED** result: actual destination/head, included source and
   retained worktree. Choose **Verify**, enter a check identity such as
   `greetings`, and `python3 -m unittest discover -v`. F5 runs visibly. Return
   to the result and confirm **Passed**. Integration alone never means verified.
7. Record observations before deliberately exercising recovery. Recovery must
   preserve the checkout and retained name/project/batch; a fresh agent does
   not restore its conversation and must receive an explicit task. Inspect
   before retrying an uncertain send to avoid duplicate work.

Do not terminate an active worker merely to test recovery. After the original
route is recorded, use a separate disposable worker for interruption/recovery.
The original updater interruption cannot be recreated honestly unless it recurs.

## Comparison receipt

| Boundary | First run | Repeat observation |
| --- | --- | --- |
| Runtime after branch switch | Missing script, exit 127 | Pending manual repeat; snapshot removal/migration passes automation |
| Chat/coordinator/worker navigation | Repeated context switching | Pending; per-client coordinator toggle passes automation |
| Form editing | Enter/arrows confusing; cursor hard to find | Pending; field navigation, multiline and cursor fixtures pass |
| Launch/recovery task | Empty Codex editor despite sent receipt | Prompt reached the editor; user still had to press Enter. Automatic submission remains broken in this live run. Recovery not repeated yet. |
| Activity | RUN persisted after completion; hooks failed | Pending new standalone worker; ancestry adapter regressions pass |
| Destination | Path pasted into branch field | Pending; branch+checkout picker and validation pass |
| Integration progress/result | Quiet line under old preview | Pending; gated Git progress and distinct actual-head result pass |
| Assembled checks | Ran outside Drudwyn; no UI receipt | Pending; visible checks and refreshed Passed receipt pass |
| Recovery continuity | Name/project/batch uncertain | Pending; retained identity and explicit missing-metadata paths pass |
| Secondary popup styling | Bland, weak hierarchy | User reports much improved appearance; asks for clearer labels, separation and contextual guidance. |

The separate two-worker extension uses `tasks/02-double.md`, short name `double`,
from the same pinned batch source. Record concurrent activity and integrate each
worker through the UI, then run all five tests in the destination. Conflict,
integration-branch promotion and cleanup are additional scenarios, not claims
about the original one-worker run. Automated acceptance covers both integration
routes with controlled agents; it does not certify live Codex behaviour.

## Runtime and rollback

The installed snapshot lives under `~/.local/share/drudwyn/runtime/current`.
Its `SOURCE` identifies the implementation commit. Installation retains earlier
snapshots. Development `main` is unchanged; no branch merge, push or release is
performed. Existing shared-daemon conversations stay running, but their hook
ownership remains unsupported until an explicit restart with `--no-daemon`.

## Live repeat feedback — guidance and discoverability

After confirming the batch, the user launched greetings. The prompt appeared in
Codex's text field, but the user had to press Enter before implementation began.
This is partial delivery success, not successful automatic submission. The cause
is not established; do not describe the startup fix as complete on this evidence.
The worker is running; completion signals and later integration are still pending.

The user finds the appearance much improved, but still needs clearer labels,
separation between elements and help explaining what to do. They do not think
they could have completed this test without the coordinating conversation.
The flow must support both first-time users and returning users who have forgotten
its controls; learning key bindings is not a substitute for discoverability.

Requested direction: a help area on the right, explaining the focused field or
command. Proposed treatment for the next UX pass:

- Show the current step and next action; distinguish selecting, previewing,
  confirming and starting. Keep advanced setup subordinate to common choices.
- Give fields visible labels, readable inactive states and clear group spacing.
  Remove duplicate batch hints; improve the faint inactive Task border.
- Update the right-side help with focus: what this choice means, a short example,
  the relevant keys, and what pressing the primary action will do.
- Explain source versus destination in everyday terms: where workers start and
  where completed changes go. Expose direct-main and integration-branch choices
  visibly instead of making a blank text field carry that decision alone.
- Provide contextual recovery guidance for missing setup and errors. Retain
  entered text and name the available next action.
- On narrow terminals, keep the same explanation reachable through a visible
  Help action or inline guidance; do not squeeze the form to retain a sidebar.

Acceptance direction: repeat launch through integration with guidance supplied
by Drudwyn itself, recording any step that still requires this chat to explain it.
These are recorded follow-up requirements/proposals, not implemented changes.
The installed test runtime remains unchanged while this live comparison runs.
