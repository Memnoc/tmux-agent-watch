# 04 — Create a shell session from Drudwyn

**Spec:** docs/specs/2026-09-30-worktree-worker-workflow.md

**Status:** done

**What to build:** Users invoke New Session from a command or the session
navigator, enter a name/directory, and arrive in a shell.

**Blocked by:** 02.
**Priority:** P1. **Stories:** 9, 10, 21.

- [x] Expose New Session alongside existing management actions and through the
  command interface; show its shortcut/help without colliding with save or kill.
- [x] Collect name and directory with an invoking-workspace directory default;
  display the resulting named shell session in navigation.
- [x] Switch only the requesting client and leave existing workers and the
  second client's selected window/session unchanged.
- [x] Cancellation, duplicate/invalid names, invalid directories, missing client,
  and command errors preserve existing sessions and allow a corrected retry.
- [x] Names/paths containing spaces or shell metacharacters are treated as data;
  creation does not implicitly create a worktree, change a branch, or start an agent.

## Working agreement

Follow the common working agreement in the approved implementation breakdown.
Use a fresh session, test through the spec’s command/UI seams, preserve existing
work, and record tests and crosscheck results before marking this ticket done.
Commit this ticket with its implementation; leave branch integration to the
coordinating session.


## Implementation receipt — 2026-09-30

- Added `session new --name NAME [--directory PATH]` and the session navigator's
  `n` New Session form. Directory defaults to the requesting terminal's selected
  workspace; Tab changes fields, Enter advances/creates, Backspace edits, and Esc
  cancels. Invalid input and command errors retain the form for correction.
  The resulting named shell is visible in both navigators. Usage and built-in
  help document the command and shortcuts; management hints wrap at narrow widths
  so New Session, Save, Kill, and Cancel stay discoverable.
- Creation resolves and pins an attached client before mutation, validates names
  and directories, and leaves duplicate detection atomic in tmux. It opens the
  configured shell directly, bypassing default-command, then switches through
  the shared client-local navigation layer. The second terminal's selection,
  existing windows/panes/processes, and Git worktree/branch state are preserved.
  A command-queue-local destroy-unattached override protects the detached shell
  until attachment, then the user's inherited cleanup policy is restored.
- Names and directories remain argv data. Tmux's own trailing-semicolon command
  parsing and name/directory format expansion are escaped explicitly. Coverage
  includes spaces, leading hyphens, substitutions, backticks, format markers,
  literal semicolons, and a default-directory suffix matching the metadata
  separator. Form fields and errors honor label redaction at 120 and 48 columns.
- Failed switching removes only the new session identified by its stable ID;
  failed cleanup reports any retained identity. An uncertain creation response
  reports that the new session may remain for inspection. A response without a
  stable ID never supplies an empty/default cleanup target. No new persistent
  registry, task content, network access, or terminal-content reading was added.
- Red/green evidence: the initial CLI test failed on missing `session`; the UI
  test failed on missing New Session; literal trailing-semicolon arguments and
  format-marker names failed in tmux parsing; inherited cleanup removed the new
  session before attachment; narrow hints clipped existing actions/Cancel;
  default-directory separator stripping changed a literal path; and lost creation
  output removed a session whose stable identity was unknown. Each regression
  now passes through the public CLI or actual tmux keys and terminal buffers.
  Invalid/duplicate names, bad directories, cancellation, missing/ambiguous/zero
  clients, injected creation/switch errors, and corrected retries also pass.
- Final verification: `cargo fmt --check`, `git diff --check`, and
  `cargo test --locked` passed (35 Rust tests). Complete `bash tests/run.sh`
  passed on the final code: 28 real two-client tests, 13 navigator tests (one
  existing optional Resurrect skip), 10 settings tests, and all launcher,
  lifecycle, privacy, packaging, and release checks. The first complete attempt
  exposed a test resize race against tmux's clipped pre-redraw frame; the test
  now waits for the resized Cancel hint, and subsequent full runs passed.
  Sandbox socket restrictions required the authorized disposable-server reruns.
- Builder self-review assessed repository standards, ticket/spec behavior,
  client identity and error paths, literal input handling, privacy/redaction,
  and legacy/native/save/kill compatibility. Per the coordinating session's
  approved workflow, an independent Northstar crosscheck follows this atomic
  implementation commit; this receipt does not claim that review is complete.

Limits: New Session is a shell-only Rust action; worktree creation, agent launch,
recovery, and batch setup remain their separate actions/tickets. Explicit relative
directories resolve from the command's working directory. Tmux and the configured
shell still own user startup hooks/configuration. Tests used only disposable
servers and repositories under `/tmp`; `main` remains `eaf2446`.
