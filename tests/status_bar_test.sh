#!/usr/bin/env bash

set -eu

ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
SOCKET="drudwyn-bar-$$"
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
tmux -L "$SOCKET" new-window -d -t bar -n opencode
codex_window="$(tmux -L "$SOCKET" display-message -p -t bar:codex '#{window_id}')"
claude_window="$(tmux -L "$SOCKET" display-message -p -t bar:claude '#{window_id}')"
opencode_window="$(tmux -L "$SOCKET" display-message -p -t bar:opencode '#{window_id}')"
tmux -L "$SOCKET" set-option -wq -t "$codex_window" @drudwyn_state working
tmux -L "$SOCKET" set-option -wq -t "$claude_window" @drudwyn_state needs_input
tmux -L "$SOCKET" set-option -wq -t "$opencode_window" @drudwyn_state done
tmux -L "$SOCKET" set-option -wq -t "$codex_window" @drudwyn_agent codex
tmux -L "$SOCKET" set-option -wq -t "$claude_window" @drudwyn_agent claude
tmux -L "$SOCKET" set-option -wq -t "$opencode_window" @drudwyn_agent opencode
tmux -L "$SOCKET" set-option -wq -t "$codex_window" @drudwyn_message 'private prompt must never render'
socket_path="$(tmux -L "$SOCKET" display-message -p '#{socket_path}')"
server_pid="$(tmux -L "$SOCKET" display-message -p '#{pid}')"
export TMUX="$socket_path,$server_pid,0"

tmux -L "$SOCKET" set-option -g @drudwyn-icon-mode safe
git init -q -b main "$TMP_DIR/example-project"
git -C "$TMP_DIR/example-project" -c user.name=Test -c user.email=test@example.invalid commit -qm initial --allow-empty
tmux -L "$SOCKET" set-option -wq -t "$codex_window" @drudwyn_context_repo "$TMP_DIR/example-project"
safe="$($ROOT/scripts/status-bar.sh bar "$codex_window" 120)"
printf '%s' "$safe" | grep -Fq '#[align=left]'
printf '%s' "$safe" | grep -Fq '#[align=centre]'
printf '%s' "$safe" | grep -Fq '#[align=right]'
printf '%s' "$safe" | grep -Fq '>>'
printf '%s' "$safe" | grep -Fq 'C '
printf '%s' "$safe" | grep -Fq 'A '
if printf '%s' "$safe" | grep -Fq 'private prompt'; then
  printf 'not ok: status bar crossed the content privacy boundary\n'; exit 1
fi
if printf '%s' "$safe" | grep -Eq '󰚩|||󰁔'; then
  printf 'not ok: safe icon mode emitted Nerd Font glyphs\n'; exit 1
fi
printf 'ok: safe bar groups windows, signals overflow, and excludes content\n'

tmux -L "$SOCKET" set-option -g @drudwyn-icon-mode nerd
tmux -L "$SOCKET" set-option -g @drudwyn-agent-icon auto
identity="$($ROOT/scripts/status-bar.sh bar "$codex_window" 160)"
printf '%s' "$identity" | grep -Fq '✣ '
printf '%s' "$identity" | grep -Fq '✦ '
printf '%s' "$identity" | grep -Fq '⌬ '
printf '%s' "$identity" | grep -Fq '#[bg=#f6c177,fg=#191724,bold]'
printf '%s' "$identity" | grep -Fq '#[bg=#3e8fb0,fg=#faf4ed,bold]'
tmux -L "$SOCKET" set-option -g @drudwyn-claude-icon 'λ'
custom_identity="$($ROOT/scripts/status-bar.sh bar "$codex_window" 160)"
printf '%s' "$custom_identity" | grep -Fq 'λ '
tmux -L "$SOCKET" set-option -gu @drudwyn-claude-icon
printf 'ok: status segments distinguish Codex, Claude, and OpenCode with per-agent overrides\n'

tmux -L "$SOCKET" set-option -g @drudwyn-agent-icon bot
nerd="$($ROOT/scripts/status-bar.sh bar "$codex_window" 120)"
printf '%s' "$nerd" | grep -Fq '#[bg=default,fg=#eb6f92,bold]●'
printf '%s' "$nerd" | grep -Fq '󰚩'
printf '%s' "$nerd" | grep -Fq '󰁔'
if printf '%s' "$nerd" | grep -Eq '|'; then
  printf 'not ok: selected workspace retained rounded powerline endcaps\n'; exit 1
fi
printf 'ok: Nerd mode renders the selected-agent and overflow vocabulary\n'

tmux -L "$SOCKET" set-option -g @drudwyn-agent-icon '󰀀'
hound="$($ROOT/scripts/status-bar.sh bar "$codex_window" 120)"
printf '%s' "$hound" | grep -Fq '󰀀'
if printf '%s' "$hound" | grep -Fq '󰚩'; then
  printf 'not ok: custom agent icon left bot icons in the bar\n'; exit 1
fi
tmux -L "$SOCKET" set-option -g @drudwyn-icon-mode safe
safe_custom="$($ROOT/scripts/status-bar.sh bar "$codex_window" 120)"
if printf '%s' "$safe_custom" | grep -Fq '󰀀'; then
  printf 'not ok: safe mode emitted the custom font glyph\n'; exit 1
fi
tmux -L "$SOCKET" set-option -g @drudwyn-icon-mode nerd
tmux -L "$SOCKET" set-option -g @drudwyn-agent-icon bot
printf 'ok: custom hound replaces bots while safe mode stays font-independent\n'

narrow="$($ROOT/scripts/status-bar.sh bar "$codex_window" 72)"
printf '%s' "$narrow" | grep -Fq '󰁔'
printf '%s' "$narrow" | grep -Fq 'example-proje'
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
printf '%s' "$selected_second" | grep -Fq '! INPUT'
if printf '%s' "$selected_second" | grep -Fq 'WORK'; then
  printf 'not ok: very narrow bar rendered another agent state before the selected agent\n'; exit 1
fi
printf 'ok: very narrow bar gives its agent state to the selected workspace\n'

for mode in safe nerd; do
  tmux -L "$SOCKET" set-option -g @drudwyn-icon-mode "$mode"
  for state in needs_input done failed; do
    case "$state" in
      needs_input) label='! INPUT' ;;
      done) label='REVIEW' ;;
      failed) label='! FAIL' ;;
    esac
    tmux -L "$SOCKET" set-option -wq -t "$claude_window" @drudwyn_state "$state"
    for width in 64 96 120 160; do
      attention_bar="$($ROOT/scripts/status-bar.sh bar "$claude_window" "$width")"
      printf '%s' "$attention_bar" | grep -Fq "$label"
    done
    # Unselected agents must explain their state in the right-hand cluster too.
    attention_bar="$($ROOT/scripts/status-bar.sh bar "$codex_window" 160)"
    printf '%s' "$attention_bar" | grep -Fq "$label"
  done
done
tmux -L "$SOCKET" set-option -wq -t "$claude_window" @drudwyn_state needs_input
printf 'ok: attention badges explain input, review, and failure in both icon modes\n'

tmux -L "$SOCKET" set-option -g @drudwyn-theme dawn
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
tmux -L "$SOCKET" set-option -wq -t "$codex_window" @drudwyn_context_repo "$repo"
git_bar="$($ROOT/scripts/status-bar.sh bar "$codex_window" 160)"
printf '%s' "$git_bar" | grep -Fq '+2'
printf '%s' "$git_bar" | grep -Fq '−0'
printf '%s' "$git_bar" | grep -Fq '2 files'
printf '%s' "$git_bar" | grep -Fq '?1'
printf 'ok: centre context reports tracked lines, files, and untracked files\n'

# User shell startup can temporarily change cwd; observe a controlled shell.
tmux -L "$SOCKET" new-window -d -t bar -n ordinary -c "$repo" bash --noprofile --norc
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
