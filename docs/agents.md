# Agent integrations

[Documentation](README.md) · [Project home](../README.md)

Agent process detection works without setup. The default Rust scanner reports
a detected live process as working; it cannot tell when that process needs input
or has finished a turn. Configure lifecycle hooks below to enable the `! INPUT`,
`REVIEW`, and `! FAIL` badges. Hooks take precedence over process detection.

| Agent              | Integration                                  | Without it           |
| ------------------ | -------------------------------------------- | -------------------- |
| Codex CLI | Lifecycle hooks in `~/.codex/hooks.json` | Process detection only |
| Claude Code        | Lifecycle hooks in `~/.claude/settings.json` | Process detection only |
| OpenCode           | Local event plugin                           | Process detection only |

<details>
<summary>Codex CLI hooks</summary>

Add these entries to the `hooks` object in `~/.codex/hooks.json`, replacing
`/path/to` with the plugin checkout. Preserve any existing hooks.

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "if [ -n \"${TMUX_PANE:-}\" ]; then /path/to/tmux-drudwyn/scripts/codex-hook.sh userPromptSubmit; fi",
            "timeout": 3
          }
        ]
      }
    ],
    "PermissionRequest": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "if [ -n \"${TMUX_PANE:-}\" ]; then /path/to/tmux-drudwyn/scripts/codex-hook.sh permissionRequest; fi",
            "timeout": 3
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "if [ -n \"${TMUX_PANE:-}\" ]; then /path/to/tmux-drudwyn/scripts/codex-hook.sh userPromptSubmit; fi",
            "timeout": 3
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "if [ -n \"${TMUX_PANE:-}\" ]; then /path/to/tmux-drudwyn/scripts/codex-hook.sh stop; fi",
            "timeout": 3
          }
        ]
      }
    ],
    "Interrupt": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "if [ -n \"${TMUX_PANE:-}\" ]; then /path/to/tmux-drudwyn/scripts/codex-hook.sh interrupt; fi",
            "timeout": 3
          }
        ]
      }
    ]
  }
}
```

Open `/hooks` in Codex to review and trust the new commands. If the running
session does not list them, restart and resume it in the same tmux pane, then
open `/hooks` again. A tmux reload does not install or trust agent hooks.
The `PostToolUse` handler restores working state after an approval completes;
it reuses the adapter's working-state command.
See the [official Codex hooks reference](https://learn.chatgpt.com/docs/hooks)
for the supported configuration schema and trust requirements.

</details>

<details>
<summary>Claude Code hooks</summary>

Hooks publish lifecycle state when Claude submits a prompt, requests approval,
or finishes. `PostToolUse` reuses the working-state command to clear the waiting
badge after a tool completes.

Merge this `hooks` object into `~/.claude/settings.json`, replacing `/path/to`
with the plugin checkout and preserving existing handlers. Use `/hooks` to
inspect the configuration; restart and resume Claude if the new hooks are not
listed. See the [Claude hooks reference](https://code.claude.com/docs/en/hooks).

```json
{
  "hooks": {
    "PostToolUse": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "if [ -n \"${TMUX_PANE:-}\" ]; then /path/to/tmux-drudwyn/scripts/claude-hook.sh UserPromptSubmit; fi",
            "timeout": 3
          }
        ]
      }
    ],
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "if [ -n \"${TMUX_PANE:-}\" ]; then /path/to/tmux-drudwyn/scripts/claude-hook.sh UserPromptSubmit; fi",
            "timeout": 3
          }
        ]
      }
    ],
    "PermissionRequest": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "if [ -n \"${TMUX_PANE:-}\" ]; then /path/to/tmux-drudwyn/scripts/claude-hook.sh PermissionRequest; fi",
            "timeout": 3
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "if [ -n \"${TMUX_PANE:-}\" ]; then /path/to/tmux-drudwyn/scripts/claude-hook.sh Stop; fi",
            "timeout": 3
          }
        ]
      }
    ],
    "StopFailure": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "if [ -n \"${TMUX_PANE:-}\" ]; then /path/to/tmux-drudwyn/scripts/claude-hook.sh StopFailure; fi",
            "timeout": 3
          }
        ]
      }
    ]
  }
}
```

</details>

<details>
<summary>OpenCode plugin</summary>
Optional. Relays OpenCode lifecycle events for immediate, exact status updates. Without it, the default scanner detects the process but cannot report input or review states.

```sh
mkdir -p ~/.config/opencode/plugins
cp /path/to/tmux-drudwyn/integrations/opencode-drudwyn.js \
  ~/.config/opencode/plugins/tmux-drudwyn.js
```

Replace `/path/to` inside the copied file with the plugin checkout.

</details>

Hooks ignore event payload content and use the inherited `TMUX_PANE` to update
the correct window. See [Privacy and local data flow](privacy.md) for the
full data boundary.
