# tmux-agent-watch

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

`tmux-agent-watch` shows which agent windows need you, without replacing the
tmux workflow you already use.

Each recognized agent gets a color-coded state in a session sidebar:

| Marker | Color | State |
| --- | --- | --- |
| `●` | blue | working |
| `●` | gold | needs input |
| `●` | green | ready for review |
| `●` | red | failed |

The plugin currently recognizes Codex, Claude Code, and OpenCode. Other windows
remain unchanged.

## Install

With [TPM](https://github.com/tmux-plugins/tpm), add this repository to
`.tmux.conf`:

```tmux
set -g @plugin 'memnoc/tmux-agent-watch'
```

While developing locally, use the absolute checkout path instead:

```tmux
run-shell /home/memnoc/tmux-agent-watch/tmux-agent-watch.tmux
```

Reload tmux, then start agents exactly as you do today. No special launcher is
required.

## Use

- `prefix + a` jumps to the oldest agent requiring attention.
- `prefix + Space` expands or collapses the left sidebar.
- `prefix + m` uses normal tmux pane zoom to temporarily hide the sidebar.
- Click an agent row in the sidebar to select its window.
- Existing window navigation and naming continue to work normally.

The three-column collapsed sidebar shows one dot per agent in stable tmux order.
The expanded view groups agents that need attention above working agents, and
adds a repository path when it clarifies the window name, semantic state, age,
plus a complete, word-wrapped summary of what the agent is doing, did, or needs.
A single sidebar pane follows the selected window within each session.

The normal horizontal window list is replaced by a single-row agent HUD. It
shows aggregate working/waiting/review counts on the left and the selected
agent's state and age on the right. Detailed summaries live in the expanded
sidebar, keeping the terminal workspace clear.

The observer checks panes every two seconds. Automatic terminal classification
is conservative by design. Exact lifecycle hooks take precedence over the
observer. Agent hooks can report an exact state with:

```sh
/path/to/tmux-agent-watch/scripts/status.sh working
/path/to/tmux-agent-watch/scripts/status.sh needs_input "Approve deployment"
/path/to/tmux-agent-watch/scripts/status.sh done "Ready for review"
/path/to/tmux-agent-watch/scripts/status.sh failed "Tests failed"
```

### Claude Code lifecycle hooks

Claude Code can publish deterministic state changes. Add these entries to the
`hooks` object in `~/.claude/settings.json`, replacing `/path/to` with the
plugin's location:

```json
{
  "UserPromptSubmit": [{
    "hooks": [{ "type": "command", "command": "/path/to/tmux-agent-watch/scripts/claude-hook.sh UserPromptSubmit" }]
  }],
  "PermissionRequest": [{
    "hooks": [{ "type": "command", "command": "/path/to/tmux-agent-watch/scripts/claude-hook.sh PermissionRequest" }]
  }],
  "Stop": [{
    "hooks": [{ "type": "command", "command": "/path/to/tmux-agent-watch/scripts/claude-hook.sh Stop" }]
  }],
  "StopFailure": [{
    "hooks": [{ "type": "command", "command": "/path/to/tmux-agent-watch/scripts/claude-hook.sh StopFailure" }]
  }]
}
```

These commands receive Claude's event JSON on standard input and use the
inherited `TMUX_PANE` to update the correct window. Outside tmux they do
nothing. Terminal parsing remains the fallback when hooks are not configured.

## Options

Set options before loading the plugin:

```tmux
set -g @agent-watch-interval 2
set -g @agent-watch-next-key a
set -g @agent-watch-sidebar-key Space
set -g @agent-watch-hud on
set -g @agent-watch-status off
set -g @agent-watch-sidebar on
set -g @agent-watch-sidebar-width 3
set -g @agent-watch-sidebar-expanded-width 38
set -g @agent-watch-working-color '#9ccfd8'
set -g @agent-watch-needs-input-color '#f6c177'
set -g @agent-watch-done-color '#a6da95'
set -g @agent-watch-failed-color '#ed8796'
```

The symbols are configurable with `@agent-watch-working-symbol`,
`@agent-watch-needs-input-symbol`, `@agent-watch-done-symbol`, and
`@agent-watch-failed-symbol`.

The sidebar and HUD are the default surfaces. The earlier tmux window list can
be retained instead of the HUD, with optional state-colored names:

```tmux
set -g @agent-watch-hud off
set -g @agent-watch-status on
set -g @agent-watch-color-window-names on
```

## Session persistence

This plugin stores live status as tmux window options. Use
`tmux-resurrect` and `tmux-continuum` for session, window, layout, and working
directory restoration.

Continuum must load after themes that replace `status-right`, or its periodic
save hook can be lost. Put it last in the TPM plugin list:

```tmux
set -g @plugin '2kabhishek/tmux2k'
set -g @plugin 'tmux-plugins/tmux-resurrect'
set -g @plugin 'tmux-plugins/tmux-continuum'
```

## Scope

Version one intentionally does not orchestrate agents, retain task history,
track tokens, or introduce its own project model. tmux remains the interface.

## License

Released under the [MIT License](LICENSE).
