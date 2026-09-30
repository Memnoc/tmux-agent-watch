# Troubleshooting

[Documentation](README.md) · [Project home](../README.md)

## Missing or old binary

If a popup says “v2 binary not found,” or newly pulled features are missing,
build from the checkout that tmux actually loads:

```sh
cd ~/.tmux/plugins/tmux-drudwyn
cargo build --release --locked
./target/release/tmux-drudwyn --version
tmux source-file ~/.tmux.conf
```

Substitute your manual checkout path if needed. Close and reopen the popup.
The package version may stay the same across development commits; a successful
rebuild is what includes the pulled changes.

The launcher prefers an explicit `DRUDWYN_V2_BIN`, then the checkout's
release build, then PATH. Check [binary selection](installation.md#which-binary-runs)
if updating `~/.local/bin` or `~/.cargo/bin` has no visible effect.

## Plugin or popup does not open

Check that the checkout exists and `~/.tmux.conf` loads it through TPM or a
`run-shell` line. With TPM, press `prefix + I` after adding the plugin, then build
the binary. Confirm your tmux supports popups:

```sh
tmux -V
tmux list-commands | rg '^display-popup'
```

If `rg` is unavailable, use `grep` for that check. Reload from inside tmux:

```sh
tmux source-file ~/.tmux.conf
```

The default prefix is `Ctrl+b`; your configuration may change it. Check it with
`tmux show-option -gv prefix`. Uppercase shortcuts require Shift.

## Shortcut opens the wrong thing

Inspect the active binding, for example:

```sh
tmux list-keys -T prefix P
tmux list-keys -T prefix w
tmux list-keys -T prefix C-s
```

Another plugin or a later configuration line can replace a binding. Set a custom
[Drudwyn shortcut](configuration.md) before loading the plugin, or resolve the
conflicting binding in your tmux configuration, then reload.

`prefix + s` opens the session navigator. `prefix + S` opens tmux's native session
tree. `prefix + Ctrl+s` is reserved for Resurrect's save binding when installed.
Remove an old `@drudwyn-native-session-key C-s` override if it takes that key.

## Agent state is missing or delayed

Start the agent inside a tmux pane. Automatic detection uses process and tmux
metadata; optional [agent integrations](agents.md) provide explicit lifecycle
transitions. Verify that hook commands point to the checkout you actually use.
If every agent stays in the plain working style, the lifecycle hooks may be
missing or untrusted. Process detection alone cannot produce input or review
badges. In Codex, open `/hooks` and review the Drudwyn commands. Newly configured
hooks report future events; they cannot recover a permission request or finished
turn that happened before they were enabled.

The default scan interval is two seconds; it can be changed in [configuration](configuration.md).

The default implementation does not read your conversation to infer progress.
For a manual refresh from the plugin directory, run:

```sh
scripts/v2.sh scan
```

## Session saving fails

Install and load tmux-resurrect first. From inside tmux, check:

```sh
tmux show-option -gqv @resurrect-save-script-path
```

An empty result means the save integration is not configured. Reload your tmux
configuration after loading Resurrect, then press `s` inside either navigator.
Read the footer for the result. Saving covers all sessions, and killing an item
does not automatically save. See [session persistence](usage.md#session-persistence).

## Worktree finish is refused

Finish accepts a linked worktree that is clean and integrated into the configured
base branch. It refuses the primary checkout, dirty worktrees, and unintegrated
branches. Review the workspace's Git state and complete your normal commit and
integration workflow, then retry. Confirm `@drudwyn-base-branch` matches your
repository. See [worktree usage](usage.md#worktrees-from-the-command-line).

## Report a bug

If these steps do not resolve it, [open an issue](https://github.com/Memnoc/tmux-drudwyn/issues)
with your OS, tmux version, plugin commit, install method, relevant configuration,
and minimal reproduction steps. Include expected and actual behavior. Share only
configuration or screenshots needed to reproduce the problem; redact private
workspace names and task content. See [contributing](../CONTRIBUTING.md).
