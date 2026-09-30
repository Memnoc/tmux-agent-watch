# Configuration

[Documentation](README.md) · [Project home](../README.md)

Defaults work without configuration. You can set overrides before loading the plugin.

Press `prefix + O` to open the Rose Pine options editor. It lists every current
Drudwyn option by category: use `j`/`k` to move, `h`/`l` or `Tab` to change
category, `Enter` to choose or edit, `e` to type a custom value, and `r` to
restore the default. The selected option has explanatory sub-text beneath the
keyboard controls describing its effect,
dependencies, and whether it affects existing views or future workspaces.
Changes apply immediately to the current tmux server, including shortcuts and
HUD/sidebar layout. Themes preview inside the editor; reopen other popups to
pick up theme, icon, or redaction changes. Colours cycle through presets with
`Enter`; `e` accepts a custom `#RRGGBB` value. Invalid choices, invalid key names,
and shortcuts already occupied by another binding are rejected. Add
the values you want to retain to `~/.tmux.conf` because tmux server options do
not survive a server restart.

### Common options

| Option                       | Default | Purpose                                    |
| ---------------------------- | ------- | ------------------------------------------ |
| `@drudwyn-agent`         | `codex` | Agent started for new worktrees            |
| `@drudwyn-base-branch`   | `main`  | Base for new tasks and finished-work validation      |
| `@drudwyn-branch-prefix` | `work/` | Prefix for generated branches              |
| `@drudwyn-theme`         | `moon`  | `rose-pine`, `moon`, or `dawn`             |
| `@drudwyn-icon-mode`     | `auto`  | Prefer Nerd Fonts when detected; `nerd` forces them, `safe` uses ASCII |
| `@drudwyn-agent-icon`    | `auto`  | Agent icon policy; `hound`, `bot`, or a custom glyph overrides all agents |
| `@drudwyn-redact-labels` | `off`   | Hide workspace labels while screen sharing |
| `@drudwyn-sidebar`       | `off`   | Enable the legacy sidebar                  |

Example:

```tmux
set -g @drudwyn-agent claude
set -g @drudwyn-base-branch trunk
set -g @drudwyn-branch-prefix quick-win/
set -g @drudwyn-theme rose-pine
```

<details>
<summary>Keys, refresh intervals, and advanced options</summary>

| Option                                | Default | Purpose                                                                |
| ------------------------------------- | ------- | ---------------------------------------------------------------------- |
| `@drudwyn-interval`               | `2`     | Agent scan interval in seconds                                         |
| `@drudwyn-git-interval`           | `10`    | Legacy Git cache refresh interval in seconds; Rust views read Git live |
| `@drudwyn-next-key`               | `a`     | Jump-to-attention key                                                  |
| `@drudwyn-sidebar-key`            | `Space` | Sidebar toggle key                                                     |
| `@drudwyn-restart-key`            | `A`     | Sidebar restart key                                                    |
| `@drudwyn-worktree-key`           | `W`     | Worktree creation key                                                  |
| `@drudwyn-finish-key`             | `X`     | Worktree finish key                                                    |
| `@drudwyn-cockpit-key`            | `P`     | Cockpit key                                                            |
| `@drudwyn-navigator-key`          | `w`     | Navigator key                                                          |
| `@drudwyn-native-navigator-key`   | `C-w`   | Native tmux tree key                                                   |
| `@drudwyn-session-key`            | `s`     | Session navigator key                                                  |
| `@drudwyn-native-session-key`     | `S`     | Native tmux session tree key                                           |
| `@drudwyn-help-key`               | `H`     | Help key                                                               |
| `@drudwyn-options-key`            | `O`     | Options editor key                                                     |
| `@drudwyn-v2`                     | `on`    | Rust implementation; `off` selects the content-reading legacy fallback |
| `@drudwyn-hud`                    | `on`    | Clustered status bar; off restores the previous tmux status layout      |
| `@drudwyn-status`                 | `off`   | Lifecycle symbols in the native tmux window list, visible with HUD off |
| `@drudwyn-sidebar-width`          | `3`     | Collapsed sidebar width                                                |
| `@drudwyn-sidebar-expanded-width` | `38`    | Expanded sidebar width                                                 |

Lifecycle colors and symbols use `@drudwyn-{working,needs-input,done,failed}-color`
and `@drudwyn-{working,needs-input,done,failed}-symbol`. Colours default to the
selected theme (`default` in the menu) and affect the clustered status bar and
native window markers. Symbols default to `●` and affect the native window list
when `@drudwyn-status` is on; the HUD uses agent identity icons instead.
`@drudwyn-color-window-names` controls lifecycle colouring of window numbers;
attention badges retain their contrasting text. Separator colour also follows
the theme unless overridden.

The HUD saves the existing global status layout before replacing it. Installs
from before this backup mechanism cannot recover the old layout automatically;
disabling those HUDs restores tmux defaults. Reload your tmux configuration to
restore a previously configured custom bar.

</details>

## Icons and font fallback

The status bar identifies supported agents with distinct single-cell symbols:
Codex `✣`, Claude `✦`, and OpenCode `⌬`. In safe mode these become `C`, `A`, and
`O`. Lifecycle color remains separate from agent identity, so the symbol says
which agent is running while its color says whether it is working, waiting,
ready for review, or failed. Waiting, review-ready, and failed agents use a
filled high-contrast badge with a text label: `! INPUT` means the agent needs
your input or approval, `REVIEW` means its turn finished, and `! FAIL` means it
failed. These labels appear alongside the agent identity in both icon modes,
including compact bars. Press `prefix + a` to jump to an agent needing attention.

The workspace navigator and cockpit details use the Drudwyn hound at `U+F0000`
when **Drudwyn Symbols** is installed, then the Nerd Font bot as a fallback.
Unknown agents in the status bar use the same generic fallback. If the required
fonts are missing or detection is unavailable, generic agent icons use `A` and
the status bar uses ASCII labels for Git and overflow.

Install a Nerd Font and select it in your terminal. Install
[`DrudwynSymbols-Regular.ttf`](../assets/brand/DrudwynSymbols-Regular.ttf) alongside
it, then restart the terminal. On Linux:

```sh
mkdir -p ~/.local/share/fonts/Drudwyn
cp assets/brand/DrudwynSymbols-Regular.ttf ~/.local/share/fonts/Drudwyn/
fc-cache -f ~/.local/share/fonts/Drudwyn
```

On macOS, open the TTF in Font Book and install it. Fonts belong on the computer
running the terminal, not just on an SSH server. The binary installer does not
install fonts for you; the TTF ships in the checkout and release archive.

Detection uses `fc-list` to check the required glyph coverage and hound family.
It establishes local font availability, not which font the terminal actually
uses. Detected SSH environments (`SSH_CONNECTION` or `SSH_TTY`) disable automatic
font assumptions. Without Fontconfig, auto mode uses ASCII. A long-running tmux
server can also have stale SSH environment values; multiple attached clients
share the global icon settings. Set overrides for the terminal you use:

```tmux
# Default: prefer the hound and Nerd Fonts, with detection-based fallback.
set -g @drudwyn-icon-mode auto
set -g @drudwyn-agent-icon auto

# Optional status-bar identity overrides.
# set -g @drudwyn-codex-icon '✣'
# set -g @drudwyn-claude-icon '✦'
# set -g @drudwyn-opencode-icon '⌬'

# To opt out of the hound while keeping detected Nerd Font symbols:
# set -g @drudwyn-agent-icon bot

# With fonts verified on a remote client or a system without Fontconfig:
# set -g @drudwyn-icon-mode nerd
# set -g @drudwyn-agent-icon hound

# If your terminal shows missing-glyph boxes:
# set -g @drudwyn-icon-mode safe
```

Explicit `nerd` and `hound` bypass detection; safe mode always wins over icon
selection. A non-`auto` `@drudwyn-agent-icon` remains the global override and
replaces every provider symbol. With the global option on `auto`, the three
provider-specific options override their individual defaults. If fallback
selects the wrong font, map `U+F0000` to **Drudwyn Symbols** in your terminal.
Reload tmux configuration and reopen existing navigator/cockpit popups to apply
icon changes.

## Upgrading existing options

The plugin imports old `@agent-watch-*` settings into `@drudwyn-*` when loaded,
including Nerd Font mode and custom symbols. Explicit new settings take precedence.
See [the upgrade guide](upgrading-to-drudwyn.md).

## Apply changes

Place options in `~/.tmux.conf` before the plugin is loaded, then run this from a terminal inside tmux:

```sh
tmux source-file ~/.tmux.conf
```

The legacy mode reads pane scrollback. See the [version boundary](privacy.md#version-boundary) before enabling it.

## Status-bar A (Rust)

The two informative rows show local tabs above selected context and labelled
GLOBAL attention. **Appearance → Visible status tabs** accepts `1` (selected
only), `3`, `4` (default), `6`, or `auto`:

```tmux
set -g @drudwyn-visible-tabs 4
set -g @drudwyn-status-key g
```

Every local window counts toward the cap, including shells and coordinators;
`+N` is outside the cap. Width may show fewer tabs to preserve names and badge
padding. The selected window remains included in local window order. Invalid
direct tab settings fall back to four. Themes, per-agent icons, safe mode,
redaction and lifecycle badge colours apply to these rows. The separator and
window-number colour controls remain relevant to legacy/native status layouts.

`prefix g` enters a status-action table: `f` Failed, `i` Input, `r` Review,
`a` all attention, `w` every window in the invoking local session; Escape cancels.
**Shortcuts → Status actions** changes the entry key. An existing unrelated
binding at the default key is preserved; choose an unused key in Options in that
case. `prefix w` and native tree bindings remain available for window movement.

Ambient jobs reuse the existing lifecycle observer; they do not start scans
for each client redraw. The context row reads only the selected checkout's
branch, never full Git details for the global fleet. A successful reconciliation
publishes its live observation time under the existing lifecycle guard. Missing,
malformed, future or expired times show **GLOBAL STALE** while retaining the
last observed counts. Expiry is twice `@drudwyn-interval` plus five seconds.
Opening Cockpit refreshes the inventory; `tmux-drudwyn scan` explicitly refreshes
the projection. Separate row jobs may observe different moments; they do not
claim an atomic two-row snapshot. Render errors replace the affected row and its
click ranges with an unavailable message.
