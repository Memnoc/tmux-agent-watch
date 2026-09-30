#!/usr/bin/env bash

set -u

pane_id="$1"
mouse_y="$2"
pane_top="$(tmux display-message -p -t "$pane_id" '#{pane_top}')"
row=$((mouse_y - pane_top))
mapping="$(tmux show-option -pqv -t "$pane_id" @drudwyn_click_map 2>/dev/null || true)"
window_id="$(printf '%s\n' "$mapping" | tr ';' '\n' | awk -F '=' -v row="$row" '$1 == row { print $2; exit }')"
[ -n "$window_id" ] || exit 0
PLUGIN_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
if [ "$(tmux show-option -gqv @drudwyn-v2)" != off ]; then
  exec "$PLUGIN_DIR/scripts/v2.sh" navigate --client "${3:-}" --window "$window_id"
fi
tmux select-window -t "$window_id"

