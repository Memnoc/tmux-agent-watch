#!/usr/bin/env bash

set -eu

ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
SOCKET="agent-watch-bar-$$"
TMP_DIR="$(mktemp -d)"

cleanup() {
  tmux -L "$SOCKET" kill-server 2>/dev/null || true
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

tmux -L "$SOCKET" -f /dev/null new-session -d -s bar -n shell
for name in northstar dots starscript extra; do
  tmux -L "$SOCKET" new-window -d -t bar -n "$name"
done
tmux -L "$SOCKET" new-window -d -t bar -n codex
tmux -L "$SOCKET" new-window -d -t bar -n claude
codex_window="$(tmux -L "$SOCKET" display-message -p -t bar:codex '#{window_id}')"
claude_window="$(tmux -L "$SOCKET" display-message -p -t bar:claude '#{window_id}')"
tmux -L "$SOCKET" set-option -wq -t "$codex_window" @agent_watch_state working
tmux -L "$SOCKET" set-option -wq -t "$claude_window" @agent_watch_state needs_input
tmux -L "$SOCKET" set-option -wq -t "$codex_window" @agent_watch_message 'private prompt must never render'
socket_path="$(tmux -L "$SOCKET" display-message -p '#{socket_path}')"
server_pid="$(tmux -L "$SOCKET" display-message -p '#{pid}')"
export TMUX="$socket_path,$server_pid,0"

safe="$($ROOT/scripts/status-bar.sh bar "$codex_window" 120)"
printf '%s' "$safe" | grep -Fq '#[align=left]'
printf '%s' "$safe" | grep -Fq '#[align=centre]'
printf '%s' "$safe" | grep -Fq '#[align=right]'
printf '%s' "$safe" | grep -Fq '>>'
printf '%s' "$safe" | grep -Fq 'A '
if printf '%s' "$safe" | grep -Fq 'private prompt'; then
  printf 'not ok: status bar crossed the content privacy boundary\n'; exit 1
fi
if printf '%s' "$safe" | grep -Eq '󰚩|||󰁔'; then
  printf 'not ok: safe icon mode emitted Nerd Font glyphs\n'; exit 1
fi
printf 'ok: safe bar groups windows, signals overflow, and excludes content\n'

tmux -L "$SOCKET" set-option -g @agent-watch-icon-mode nerd
nerd="$($ROOT/scripts/status-bar.sh bar "$codex_window" 120)"
printf '%s' "$nerd" | grep -Fq '▶'
printf '%s' "$nerd" | grep -Fq '󰚩'
printf '%s' "$nerd" | grep -Fq '󰁔'
if printf '%s' "$nerd" | grep -Eq '|'; then
  printf 'not ok: selected workspace retained rounded powerline endcaps\n'; exit 1
fi
printf 'ok: Nerd mode renders the selected-agent and overflow vocabulary\n'

narrow="$($ROOT/scripts/status-bar.sh bar "$codex_window" 72)"
printf '%s' "$narrow" | grep -Fq '󰁔'
printf '%s' "$narrow" | grep -Fq 'tmux-agent-wat'
printf '%s' "$narrow" | grep -Fq 'WORK'
if printf '%s' "$narrow" | grep -Fq 'northstar'; then
  printf 'not ok: narrow bar retained verbose inactive workspace labels\n'; exit 1
fi
printf 'ok: narrow bar collapses labels while preserving overflow navigation\n'

split_width="$($ROOT/scripts/status-bar.sh bar "$codex_window" 96)"
if printf '%s' "$split_width" | grep -Eq 'shell|nort|dots'; then
  printf 'not ok: split-width bar retained inactive workspace labels that collide with context\n'; exit 1
fi
printf 'ok: split-width bar reserves its centre for context and selected-agent identity\n'

very_narrow="$($ROOT/scripts/status-bar.sh bar "$codex_window" 64)"
if printf '%s' "$very_narrow" | grep -Eq '#\[align=(centre|right)\]'; then
  printf 'not ok: very narrow bar retained competing alignment regions\n'; exit 1
fi
printf 'ok: very narrow bar renders one collision-free sequential stream\n'

selected_second="$($ROOT/scripts/status-bar.sh bar "$claude_window" 64)"
printf '%s' "$selected_second" | grep -Fq 'WAIT'
if printf '%s' "$selected_second" | grep -Fq 'WORK'; then
  printf 'not ok: very narrow bar rendered another agent state before the selected agent\n'; exit 1
fi
printf 'ok: very narrow bar gives its agent state to the selected workspace\n'

tmux -L "$SOCKET" set-option -g @agent-watch-theme dawn
dawn="$($ROOT/scripts/status-bar.sh bar "$codex_window" 120)"
printf '%s' "$dawn" | grep -Fq '#907aa9'
printf '%s' "$dawn" | grep -Fq '#56949f'
printf 'ok: status bar follows the selected Rose Pine palette variant\n'

repo="$TMP_DIR/repo"
git init -q "$repo"
git -C "$repo" config user.name Test
git -C "$repo" config user.email test@example.invalid
printf 'one\n' > "$repo/file.txt"
git -C "$repo" add file.txt
git -C "$repo" commit -qm initial
printf 'two\nthree\n' >> "$repo/file.txt"
printf 'new\n' > "$repo/new.txt"
tmux -L "$SOCKET" set-option -wq -t "$codex_window" @agent_watch_context_repo "$repo"
git_bar="$($ROOT/scripts/status-bar.sh bar "$codex_window" 160)"
printf '%s' "$git_bar" | grep -Fq '+2'
printf '%s' "$git_bar" | grep -Fq '−0'
printf '%s' "$git_bar" | grep -Fq '2 files'
printf '%s' "$git_bar" | grep -Fq '?1'
printf 'ok: centre context reports tracked lines, files, and untracked files\n'

tmux -L "$SOCKET" new-window -d -t bar -n ordinary -c "$repo"
ordinary_window="$(tmux -L "$SOCKET" display-message -p -t bar:ordinary '#{window_id}')"
ordinary_bar="$($ROOT/scripts/status-bar.sh bar "$ordinary_window" 160)"
if ! printf '%s' "$ordinary_bar" | grep -Fq '+2'; then
  printf 'not ok: ordinary Git workspace left the centre context blank\n'; exit 1
fi
printf 'ok: ordinary Git workspace receives centre context without agent metadata\n'

very_narrow="$($ROOT/scripts/status-bar.sh bar "$codex_window" 48)"
printf '%s' "$very_narrow" | grep -Fq 'repo'
printf '%s' "$very_narrow" | grep -Fq "$(git -C "$repo" branch --show-current)"
printf '%s' "$very_narrow" | grep -Fq '●'
printf '%s' "$very_narrow" | grep -Fq 'WORK'
printf '%s' "$very_narrow" | grep -Fq '󰁔'
printf 'ok: very narrow bar preserves project, Git, agent, and navigation signals\n'
