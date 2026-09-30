#!/usr/bin/env bash

set -u

control() {
  local key
  key="$(tmux show-option -gqv "$1" 2>/dev/null || true)"
  printf '    %-7s%s\n' "${key:-$2}" "$3"
}

printf '\033[H\033[J'
printf '  tmux-drudwyn\n\n'
printf '  AGENTS\n'
printf '    ●  normal agent\n'
printf '    ◆  agent in a linked Git worktree\n\n'
printf '  CONTROLS\n'
control @drudwyn-next-key a 'jump to the oldest agent needing attention'
control @drudwyn-cockpit-key P 'open the workspace cockpit'
control @drudwyn-worktree-key W 'create a linked worktree and start an agent'
control @drudwyn-finish-key X 'finish a clean, merged linked worktree'
printf '    m      zoom or unzoom the current tmux pane\n'
control @drudwyn-navigator-key w 'open the grouped workspace navigator'
control @drudwyn-native-navigator-key C-w 'open the native tmux window tree'
control @drudwyn-session-key s 'open the compact session navigator'
control @drudwyn-native-session-key S 'open the native tmux session tree'
control @drudwyn-options-key O 'customise Drudwyn options'
printf '    C-s    save all sessions (tmux-resurrect)\n\n'
printf '  INSIDE NAVIGATORS\n'
printf '    n      new shell session in the session navigator (Tab edits, Esc cancels)\n'
printf '    s      save all sessions with tmux-resurrect\n'
printf '    r      rename selected window or session (Enter applies, Esc cancels)\n'
printf '    x      kill selected workspace or session\n'
printf '    y      confirm kill; Esc or n cancels\n\n'
printf '  STATUS LINE\n'
printf '    left   ordinary workspaces; chevron means more\n'
printf '    centre current Git changes; never prompt or response content\n'
printf '    right  managed agents coloured by lifecycle state\n\n'
printf '  WORKTREES\n'
printf '    CLEAN  no uncommitted changes\n'
printf '    DIRTY  has uncommitted changes; review, commit, or discard them\n\n'
printf '  [q/Esc] Close\n'

while IFS= read -rsn1 key; do
  case "$key" in
    q|$'\033') exit 0 ;;
  esac
done
