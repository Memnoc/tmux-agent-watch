# tmux-agent-watch

`tmux-agent-watch` shows which agent windows need you, without replacing the
tmux workflow you already use.

Each recognized agent gets a small state marker in the existing window list:

| Marker | State |
| --- | --- |
| `●` | working |
| `?` | needs input |
| `✓` | ready for review |
| `!` | failed |

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
- `prefix + A` opens the agent overview. Enter a row number to focus it.
- Existing window navigation and naming continue to work normally.

The observer checks panes every two seconds. Automatic terminal classification
is conservative by design. Agent hooks can report an exact state with:

```sh
/path/to/tmux-agent-watch/scripts/status.sh working
/path/to/tmux-agent-watch/scripts/status.sh needs_input "Approve deployment"
/path/to/tmux-agent-watch/scripts/status.sh done "Ready for review"
/path/to/tmux-agent-watch/scripts/status.sh failed "Tests failed"
```

## Options

Set options before loading the plugin:

```tmux
set -g @agent-watch-interval 2
set -g @agent-watch-next-key a
set -g @agent-watch-overview-key A
set -g @agent-watch-working-color '#9ccfd8'
set -g @agent-watch-needs-input-color '#f6c177'
set -g @agent-watch-done-color '#a6da95'
set -g @agent-watch-failed-color '#ed8796'
```

The symbols are configurable with `@agent-watch-working-symbol`,
`@agent-watch-needs-input-symbol`, `@agent-watch-done-symbol`, and
`@agent-watch-failed-symbol`.

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

