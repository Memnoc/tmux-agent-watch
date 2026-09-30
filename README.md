<p align="center">
  <img src="docs/images/brand/drudwyn-logo-v3.png" width="120" alt="Drudwyn: an attentive hound in a pastel-edged badge.">
</p>

<h1 align="center">Drudwyn</h1>

<p align="center">
  <strong>Keep every agent within reach.</strong><br>
  A tmux plugin for navigating coding agents, tracking work trees and workspaces.
</p>

<p align="center">
  <img src="docs/images/design/hero-workflow.png" width="100%" alt="From projects and agents to a live status bar, workspace navigation, and safe worktree cleanup—all inside tmux.">
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-c4a7e7" alt="License: MIT"></a>
  <a href="#install"><img src="https://img.shields.io/badge/platforms-Linux_%C2%B7_macOS-9ccfd8" alt="Platforms: Linux and macOS"></a>
</p>

<p align="center">
  <a href="#install"><strong>Install</strong></a> ·
  <a href="#how-it-works">Overview</a> ·
  <a href="#use">Shortcuts</a> ·
  <a href="#agent-integrations">Agents</a> ·
  <a href="docs/README.md">Docs</a> ·
  <a href="https://github.com/Memnoc/tmux-drudwyn/releases">Releases</a> ·
  <a href="docs/privacy.md">Privacy</a>
</p>

<p align="center">
  Works with <strong>Codex · Claude Code · OpenCode</strong><br>
  Live agent state and Git context across tmux sessions, projects, and worktrees.
</p>

---

## Install

You need tmux with popup support, Git, Bash, and a supported agent installed.
This checkout uses the v2 Rust implementation; building it requires Rust and Cargo.
`prefix` means your tmux prefix key—normally `Ctrl+b`, followed by the indicated key.

**Fonts:** Drudwyn prefers Nerd Font symbols and its hound icon by default.
For that appearance, select a [Nerd Font](https://www.nerdfonts.com/) in your
terminal and install the bundled [Drudwyn Symbols font](assets/brand/DrudwynSymbols-Regular.ttf).
Restart your terminal after installing it. The font must be installed on the
machine displaying the terminal, including when you connect over SSH.

Automatic mode checks local fonts with Fontconfig (`fc-list`): **hound → bot
if the hound font is missing → `A` if Nerd Font support is missing or unknown**.
It cannot verify the active terminal's font or a remote client's fonts.
Without Fontconfig (including many macOS setups), or over detected SSH sessions,
it uses `A`; after verifying your fonts, select `nerd` and `hound` explicitly.
[Font installation, overrides, and troubleshooting](docs/configuration.md#icons-and-font-fallback).

**1. Add the plugin.** With [TPM](https://github.com/tmux-plugins/tpm), add this to `~/.tmux.conf` before TPM is loaded:

```tmux
set -g @plugin 'memnoc/tmux-drudwyn'
```

Reload your configuration, then press `prefix + I` to install the plugin:

```sh
tmux source-file ~/.tmux.conf
```

**2. Build this checkout.** TPM installs the scripts but does not compile the binary:

```sh
cargo build --release --locked --manifest-path ~/.tmux/plugins/tmux-drudwyn/Cargo.toml
```

**3. Open the cockpit.** Reload from a terminal inside tmux:

```sh
tmux source-file ~/.tmux.conf
```

Press `prefix + P`. You should see the Workspace Cockpit. Press `Esc` to close it,
then `prefix + w` to find and switch to an existing window. Start agents normally;
no special launcher is required.

[Other install methods and updates](docs/installation.md) · [Troubleshooting](docs/troubleshooting.md)

## How it works

The status bar shows agent state and Git context. The navigators take you to a
session or workspace. The cockpit helps you start, review, and finish work.

<img src="docs/images/navigator-cockpit-surfaces.png" alt="Session Navigator, Workspace Navigator, and Workspace Cockpit shown side by side.">

When an agent needs input, press `prefix + a` to jump to the oldest agent needing
attention, or `prefix + w` to choose a workspace. Review and continue the task in
its original pane. When a linked worktree is clean and integrated into the base
branch, `prefix + X` can remove it while retaining its branch.

The default Rust implementation never reads prompts, responses, or terminal
scrollback. It uses lifecycle and Git metadata. [Privacy and data flow](docs/privacy.md).

## Use

| Key | Action |
| --- | --- |
| `prefix + P` | Open the Workspace Cockpit |
| `prefix + w` | Find a workspace across sessions |
| `prefix + s` | Find a session |
| `prefix + a` | Jump to an agent needing attention |
| `prefix + W` | Create a worktree and start an agent |
| `prefix + X` | Finish a clean, integrated worktree |
| `prefix + O` | Customise Drudwyn options |
| `prefix + H` | Open help |

Inside either navigator: `/` filters, `Enter` switches, `r` renames, `x` asks to
kill the selection, and `s` saves all sessions through tmux-resurrect.
`Esc` or `q` closes the navigator.

[Daily use, all shortcuts, and worktree commands](docs/usage.md)

## Agent integrations

Codex, Claude Code, and OpenCode are detected automatically. Configure lifecycle
hooks to show when an agent needs input or is ready for review.
[Set up your agent](docs/agents.md).

## Configure

Press `prefix + O` to choose an agent, theme, behaviour, lifecycle style, or
shortcut. Changes apply immediately to the current tmux server; copy the values
you want to keep into `~/.tmux.conf`. Defaults work without extra configuration.
[Configuration reference](docs/configuration.md).

Prefer the bot? Add `set -g @drudwyn-agent-icon bot` to `~/.tmux.conf`.

## Session persistence

Use tmux-resurrect to save and restore sessions. Press `s` inside a navigator to
save; its footer reports the result. [Session persistence guide](docs/usage.md#session-persistence).

## Worktrees from the command line

Automate workspace creation and removal with the bundled scripts.
[Commands and safeguards](docs/usage.md#worktrees-from-the-command-line).

## Maintainers

[Contributing and development](CONTRIBUTING.md) · [Release preflight](docs/preflight-checklist.md) · [Architecture decisions](docs/adr/README.md)

## Scope

Tmux remains the interface and process supervisor. The plugin does not retain
task history or track tokens. Employee monitoring, productivity scoring, and
decisions about people are outside its intended purpose. [Privacy details](docs/privacy.md).

## License

Released under the [MIT License](LICENSE).

The logo is AI-generated. [Artwork provenance](assets/brand/README.md).
