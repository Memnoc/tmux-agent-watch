#!/usr/bin/env bash

set -u

printf '\033[H\033[J'
printf '  tmux-agent-watch\n\n'
printf '  AGENTS\n'
printf '    ●  normal agent\n'
printf '    ◆  agent in a linked Git worktree\n\n'
printf '  CONTROLS\n'
printf '    a      jump to the oldest agent needing attention\n'
printf '    Space  expand or collapse the agent sidebar\n'
printf '    A      restart a stuck sidebar\n'
printf '    P      open the workspace cockpit\n'
printf '    W      create a linked worktree and start an agent\n'
printf '    X      finish a clean, merged linked worktree\n'
printf '    m      zoom or unzoom the current tmux pane\n'
printf '    w      open the tmux window chooser\n\n'
printf '  WORKTREES\n'
printf '    CLEAN  no uncommitted changes\n'
printf '    DIRTY  has uncommitted changes; review, commit, or discard them\n\n'
printf '  Press q or Escape to close.\n'

while IFS= read -rsn1 key; do
  case "$key" in
    q|$'\033') exit 0 ;;
  esac
done
