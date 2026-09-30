#!/usr/bin/env bash

set -u

PLUGIN_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
current="$(tmux list-clients -F '#{client_name}|#{window_id}' | awk -F '|' -v client="${1:-}" '$1 == client {print $2}')"
target="$({
  tmux list-windows -a -F '#{@drudwyn_attention_since}|#{window_id}|#{session_name}' |
    awk -F '|' -v current="$current" '$1 != "" && $2 != current { print }' |
    sort -n
  tmux list-windows -a -F '#{@drudwyn_attention_since}|#{window_id}|#{session_name}' |
    awk -F '|' -v current="$current" '$1 != "" && $2 == current { print }'
} | head -n 1)"

if [ -z "$target" ]; then
  tmux display-message 'No agents need attention'
  exit 0
fi

window_id="$(printf '%s' "$target" | cut -d '|' -f2)"
session="$(printf '%s' "$target" | cut -d '|' -f3)"
if [ "$(tmux show-option -gqv @drudwyn-v2)" != off ]; then
  exec "$PLUGIN_DIR/scripts/v2.sh" navigate --client "${1:-}" --window "$window_id"
fi
tmux switch-client -t "$session"
tmux select-window -t "$window_id"
