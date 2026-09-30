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

## Review correction receipt — 2026-09-30

- Reproduced the P2 review finding with the public New Session command and two
  attached clients: names containing a literal backslash, with or without a
  trailing semicolon, returned `no such session` under both inherited
  `destroy-unattached` modes. The four-case regression went red before the fix.
- A disposable native tmux probe confirmed that `new-session -s` stores each
  backslash doubled. Its
  [name normalization](https://github.com/tmux/tmux/blob/3.4/session.c#L221-L237)
  explains why the queued raw-name cleanup target failed. The initial target now
  uses that native stored spelling; creation still receives the user's name as
  argument data, and all later navigation, restoration, and cleanup use the
  returned stable session ID. No valid name is newly rejected. Usage documents
  tmux's native spelling instead of promising byte-identical backend storage.
- The regression checks both cleanup modes, backslash names with/without a
  trailing semicolon, a directory ending in literal backslash-semicolon,
  canonical stored names, duplicate rejection, exactly one new session,
  requester attachment, the other terminal's unchanged selection, existing
  windows/panes/processes, unchanged global cleanup policy, and restoration of
  inherited session policy. All 12 focused New Session tests pass.
- `cargo fmt --check`, `cargo test --locked` (35 Rust tests), and
  `git diff --check` passed. Complete `bash tests/run.sh` passed on the final
  code: 29 real two-client tests, 13 navigator tests (one existing optional
  Resurrect skip), 10 settings tests, and all launcher, lifecycle, privacy,
  packaging, and release checks. Fixtures used only disposable servers and
  repositories under `/tmp`; local tmux sockets required sandbox escalation.
- Builder self-review covered normalization, exact targeting before the ID is
  available, literal-input handling, duplicate errors, and privacy. A fresh
  independent review follows this correction commit in the coordinating session.
  No later ticket, specification, or `main` change is included.

## Independent re-review receipt — 2026-09-30

- Reviewed correction `06db30e...533305a`, with the complete ticket change
  `825be67...533305a` as context. Applied the crosscheck Standards and Spec axes
  sequentially in a fresh reviewer session against this ticket, its approved
  specification, `CONTRIBUTING.md`, domain/architecture guidance, and the privacy
  boundary.
- **Standards: 0 findings.** The normalization is confined to the initial exact
  tmux target. Creation retains the literal input; navigation, error cleanup,
  and policy restoration retain stable session IDs. The correction adds no
  content inspection, persistence, or network behavior. No actionable baseline
  code smell or documented-standard violation was found.
- **Spec: 0 findings.** The original P2 backslash-name failure is resolved.
  Names with and without trailing semicolons succeed under both inherited
  cleanup policies; duplicate rejection, requester-only attachment, existing
  sessions/windows/panes/processes, literal directory input, and inherited
  policy restoration are covered. Usage accurately describes tmux's native
  stored spelling, and the correction introduces no new targeting behavior
  outside the required initial name lookup.
- Independently ran `cargo build --locked --offline`, all 12 focused New Session
  real two-client tests (`python3 tests/independent_navigation_test.py -k
  new_session -v`), `cargo fmt --check`, `cargo test --locked --offline`
  (35 Rust tests), and `git diff --check`: all passed. The initial sandbox run
  could not create local tmux sockets; the authorized isolated rerun passed on
  tmux 3.4 using disposable servers and repositories under `/tmp`.
- The builder's complete-suite result above was inspected, not rerun for this
  focused correction review. No residual finding requires a broader rerun.
  This receipt is the only reviewer change; `main` remains `eaf2446`.
