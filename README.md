# tmux-agent-watch

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

See every coding agent that needs you—across tmux sessions, projects, and Git
worktrees—without reading transcripts or replacing your tmux workflow.

`tmux-agent-watch` recognizes Codex, Claude Code, and OpenCode. It adds
content-blind lifecycle state, Git context, navigation, and guarded worktree
actions to tmux.

## Install

### 1. Add the plugin

With [TPM](https://github.com/tmux-plugins/tpm), add this to `.tmux.conf`:

```tmux
set -g @plugin 'memnoc/tmux-agent-watch'
```

Install plugins with `prefix + I`, or reload tmux if the plugin is already
checked out.

### 2. Install the binary

TPM does not compile Rust. Run the bundled installer once after installing or
updating the plugin:

```sh
~/.tmux/plugins/tmux-agent-watch/install.sh
```

It downloads the correct Linux or macOS build, verifies its checksum, and
installs it to `~/.local/bin`.

To build from source instead:

```sh
cargo install --locked --path ~/.tmux/plugins/tmux-agent-watch
```

Reload tmux, then press `prefix + P` to open the Workspace Cockpit. Agents can
still be started normally; no special launcher is required.

## How it works

<img src="docs/images/design/hero-workflow.png" alt="Diagram: many projects and agents feed a content-blind status bar, which feeds the Workspace Navigator and Cockpit, which finish work safely back to a clean, merged worktree">

| Surface | What it answers | Open it |
| --- | --- | --- |
| Status bar | Where am I, what changed, and who needs me? | Always visible |
| Workspace Navigator | What is running across all tmux sessions? | `prefix + w` |
| Workspace Cockpit | What should I review, open, start, or finish? | `prefix + P` |

### Status at a glance

<img src="docs/images/design/status-bar-anatomy.png" alt="Annotated tmux status bar split into three zones: ordinary workspaces on the left, content-blind Git context in the centre, and lifecycle-colored agents on the right, with a variant showing the active-workspace highlight">

| Color | State | Meaning |
| --- | --- | --- |
| Blue | Working | The agent is active |
| Gold | Waiting | The agent needs input |
| Green | Review | Work is ready to inspect |
| Red | Failed | The agent or task failed |

Only lifecycle state, branch, and Git counts are shown. Prompts, responses, and
terminal scrollback are never read.

### Find work, then act

<img src="docs/images/design/navigator-cockpit-surfaces.png" alt="Side-by-side comparison of the Workspace Navigator, which lists every window and agent across tmux sessions, and the Workspace Cockpit, which shows full detail and start, review, jump, and finish actions for the selected one">

| Cockpit action | Result | Direct key |
| --- | --- | --- |
| Start | Create a linked worktree, choose an agent, and begin the task | `prefix + W` |
| Review | Show agents waiting, failed, or ready for review | — |
| Jump | Open a live agent workspace | `prefix + w` |
| Finish | Remove a clean, integrated worktree while retaining its branch | `prefix + X` |

<img src="docs/images/design/lifecycle-flow.png" alt="Lifecycle flow diagram: start task creates a worktree, the agent moves to working, then needs attention when waiting or failed, then review when idle and ready, then finish safely once clean and merged">

Finish refuses primary checkouts, dirty worktrees, and branches not integrated
into the configured base branch.

### Responsive layout

<img src="docs/images/design/responsive-layouts.png" alt="The status bar at three widths: wide showing every workspace and agent, narrow collapsing inactive workspace labels to an arrow, and compact under 80 columns sharing project, Git, lifecycle, and navigator cues in one flow">

Workspace, Git, lifecycle, and navigation context remain visible as the
terminal narrows; labels collapse before information collides.

## Use

| Key | Action |
| --- | --- |
| `prefix + H` | Open help (`q` or Escape closes it) |
| `prefix + P` | Open the Workspace Cockpit |
| `prefix + w` | Open the grouped workspace navigator |
| `prefix + C-w` | Open tmux's native window tree |
| `prefix + a` | Jump to the oldest agent needing attention |
| `prefix + W` | Create a worktree and start an agent |
| `prefix + X` | Finish the selected clean, integrated worktree |
| `prefix + Space` | Toggle the optional legacy sidebar |
| `prefix + A` | Recreate a stuck legacy sidebar |

Existing tmux window navigation, naming, and pane zoom continue to work.

## Agent integrations

Automatic terminal classification works without setup. Optional hooks provide
exact lifecycle transitions and take precedence over observation.

| Agent | Integration | Without it |
| --- | --- | --- |
| Codex CLI 0.151.0+ | Lifecycle hooks in `~/.codex/config.toml` | Terminal observation |
| Claude Code | Lifecycle hooks in `~/.claude/settings.json` | Terminal observation |
| OpenCode | Local event plugin | Terminal observation |

<details>
<summary>Codex CLI hooks</summary>

Replace `/path/to` with the plugin checkout:

```toml
[hooks]
userPromptSubmit = [{ type = "command", command = "/path/to/tmux-agent-watch/scripts/codex-hook.sh userPromptSubmit" }]
permissionRequest = [{ type = "command", command = "/path/to/tmux-agent-watch/scripts/codex-hook.sh permissionRequest" }]
stop = [{ type = "command", command = "/path/to/tmux-agent-watch/scripts/codex-hook.sh stop" }]
interrupt = [{ type = "command", command = "/path/to/tmux-agent-watch/scripts/codex-hook.sh interrupt" }]
```

Review and trust the commands when Codex prompts you.

</details>

<details>
<summary>Claude Code hooks</summary>

Add this inside the `hooks` object in `~/.claude/settings.json`, replacing
`/path/to` with the plugin checkout:

```json
{
  "UserPromptSubmit": [{ "hooks": [{ "type": "command", "command": "/path/to/tmux-agent-watch/scripts/claude-hook.sh UserPromptSubmit" }] }],
  "PermissionRequest": [{ "hooks": [{ "type": "command", "command": "/path/to/tmux-agent-watch/scripts/claude-hook.sh PermissionRequest" }] }],
  "Stop": [{ "hooks": [{ "type": "command", "command": "/path/to/tmux-agent-watch/scripts/claude-hook.sh Stop" }] }],
  "StopFailure": [{ "hooks": [{ "type": "command", "command": "/path/to/tmux-agent-watch/scripts/claude-hook.sh StopFailure" }] }]
}
```

</details>

<details>
<summary>OpenCode plugin</summary>

```sh
mkdir -p ~/.config/opencode/plugins
cp /path/to/tmux-agent-watch/integrations/opencode-agent-watch.js \
  ~/.config/opencode/plugins/tmux-agent-watch.js
```

Replace `/path/to` inside the copied file with the plugin checkout.

</details>

Hooks ignore event payload content and use the inherited `TMUX_PANE` to update
the correct window. See [Privacy and local data flow](docs/privacy.md) for the
full data boundary.

## Configure

Defaults work without configuration. Set overrides before loading the plugin.

### Common options

| Option | Default | Purpose |
| --- | --- | --- |
| `@agent-watch-agent` | `codex` | Agent started for new worktrees |
| `@agent-watch-base-branch` | `main` | Branch used to validate finished work |
| `@agent-watch-branch-prefix` | `work/` | Prefix for generated branches |
| `@agent-watch-theme` | `moon` | `rose-pine`, `moon`, or `dawn` |
| `@agent-watch-icon-mode` | `safe` | Use `nerd` for Nerd Font icons |
| `@agent-watch-redact-labels` | `off` | Hide workspace labels while screen sharing |
| `@agent-watch-sidebar` | `off` | Enable the legacy sidebar |

Example:

```tmux
set -g @agent-watch-agent claude
set -g @agent-watch-base-branch trunk
set -g @agent-watch-branch-prefix quick-win/
set -g @agent-watch-theme rose-pine
```

<details>
<summary>Keys, refresh intervals, and advanced options</summary>

| Option | Default | Purpose |
| --- | --- | --- |
| `@agent-watch-interval` | `2` | Agent scan interval in seconds |
| `@agent-watch-git-interval` | `10` | Git refresh interval in seconds |
| `@agent-watch-next-key` | `a` | Jump-to-attention key |
| `@agent-watch-sidebar-key` | `Space` | Sidebar toggle key |
| `@agent-watch-restart-key` | `A` | Sidebar restart key |
| `@agent-watch-worktree-key` | `W` | Worktree creation key |
| `@agent-watch-finish-key` | `X` | Worktree finish key |
| `@agent-watch-cockpit-key` | `P` | Cockpit key |
| `@agent-watch-navigator-key` | `w` | Navigator key |
| `@agent-watch-native-navigator-key` | `C-w` | Native tmux tree key |
| `@agent-watch-help-key` | `H` | Help key |
| `@agent-watch-v2` | `on` | Rust implementation; `off` selects legacy Bash |
| `@agent-watch-hud` | `on` | Lifecycle HUD |
| `@agent-watch-status` | `off` | Legacy status display |
| `@agent-watch-sidebar-width` | `3` | Collapsed sidebar width |
| `@agent-watch-sidebar-expanded-width` | `38` | Expanded sidebar width |

Lifecycle colors and symbols use `@agent-watch-{working,needs-input,done,failed}-color`
and `@agent-watch-{working,needs-input,done,failed}-symbol`.

</details>

## Worktrees from the command line

The cockpit is the normal interface, but the underlying scripts are available:

```sh
scripts/worktree-new.sh feature/auth opencode
scripts/worktree-new.sh --repo /path/to/repository feature/auth opencode
scripts/worktree-remove.sh feature/auth
```

Extra arguments after the branch are used as the exact agent command. Set
`AGENT_WATCH_WORKTREE_ROOT` to change the worktree parent directory. Removal
refuses dirty worktrees and retains the branch.

## Session persistence

Live state is stored in tmux window options. Use `tmux-resurrect` and
`tmux-continuum` to restore sessions, windows, layouts, and working directories.
Load Continuum after themes that replace `status-right`.

## Maintainers

Release tags must match the Cargo package version. GitHub Actions builds and
checksums Linux and macOS archives for x86_64 and ARM64. Before tagging, run the
release workflow manually and complete the
[preflight checklist](docs/preflight-checklist.md) against its `release-bundle`.

## Scope

The plugin does not retain task history, track tokens, or create a separate
project model. tmux remains the interface and process supervisor.

## License

Released under the [MIT License](LICENSE).
