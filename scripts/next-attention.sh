#!/usr/bin/env bash

set -u

current="$(tmux display-message -p '#{window_id}')"
target="$({
  tmux list-windows -a -F '#{@agent_watch_attention_since}|#{window_id}|#{session_name}' |
    awk -F '|' -v current="$current" '$1 != "" && $2 != current { print }' |
    sort -n
  tmux list-windows -a -F '#{@agent_watch_attention_since}|#{window_id}|#{session_name}' |
    awk -F '|' -v current="$current" '$1 != "" && $2 == current { print }'
} | head -n 1)"

if [ -z "$target" ]; then
  tmux display-message 'No agents need attention'
  exit 0
fi

window_id="$(printf '%s' "$target" | cut -d '|' -f2)"
session="$(printf '%s' "$target" | cut -d '|' -f3)"
tmux switch-client -t "$session"
tmux select-window -t "$window_id"
