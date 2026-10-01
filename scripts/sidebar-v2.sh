#!/usr/bin/env bash

set -u
PLUGIN_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
session="$1"
previous=''
trap 'previous=""' USR1

while tmux has-session -t "$session" 2>/dev/null; do
  current="$(tmux display-message -p -t "$session:" '#{window_id}')"
  expanded="$(tmux show-option -qv -t "$session" @drudwyn_sidebar_expanded 2>/dev/null || true)"
  theme="$(tmux show-option -gqv @drudwyn-theme 2>/dev/null || true)"
  args=(sidebar "$session" "$current" --theme "${theme:-rose-pine}")
  [ "$expanded" = on ] && args+=(--expanded)
  output="$($PLUGIN_DIR/scripts/v2.sh "${args[@]}")"
  click_map="${output##*$'\034'}"
  frame="${output%$'\034'*}"
  tmux set-option -pq -t "$TMUX_PANE" @drudwyn_click_map "$click_map"
  if [ "$frame" != "$previous" ]; then
    printf '\033[H\033[J%s' "$frame"
    previous="$frame"
  fi
  sleep 1
done
