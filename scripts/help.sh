#!/usr/bin/env bash

set -u

printf '\033[H\033[J'
printf '  tmux-agent-watch\n\n'
printf '  AGENTS\n'
printf '    ●  normal agent\n'
printf '    ◆  agent in a linked Git worktree\n\n'
printf '  CONTROLS\n'
printf '    a      jump to the oldest agent needing attention\n'
printf '    P      open the workspace cockpit\n'
printf '    W      create a linked worktree and start an agent\n'
printf '    X      finish a clean, merged linked worktree\n'
printf '    m      zoom or unzoom the current tmux pane\n'
printf '    w      open the grouped workspace navigator\n'
printf '    C-w    open the native tmux window tree\n'
printf '    s      open the compact session navigator\n'
printf '    C-s    open the native tmux session tree\n\n'
printf '  STATUS LINE\n'
printf '    left   ordinary workspaces; chevron means more\n'
printf '    centre current Git changes; never prompt or response content\n'
printf '    right  managed agents coloured by lifecycle state\n\n'
printf '  WORKTREES\n'
printf '    CLEAN  no uncommitted changes\n'
printf '    DIRTY  has uncommitted changes; review, commit, or discard them\n\n'
printf '  Press q or Escape to close.\n'

while IFS= read -rsn1 key; do
  case "$key" in
    q|$'\033') exit 0 ;;
  esac
done
