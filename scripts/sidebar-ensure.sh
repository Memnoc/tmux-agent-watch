#!/usr/bin/env bash

set -u
source "$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)/lib.sh"

[ "$(tmux_option @agent-watch-sidebar on)" = on ] || exit 0

target="${1:-${TMUX_PANE:-}}"
[ -n "$target" ] || target="$(tmux display-message -p '#{pane_id}' 2>/dev/null || true)"
[ -n "$target" ] || exit 0

session="$(tmux display-message -p -t "$target" '#{session_name}' 2>/dev/null || true)"
target_pane="$(tmux display-message -p -t "$target" '#{pane_id}' 2>/dev/null || true)"
target_window="$(tmux display-message -p -t "$target" '#{window_id}' 2>/dev/null || true)"
[ -n "$session" ] && [ -n "$target_pane" ] || exit 0
target_name="$(tmux display-message -p -t "$target_window" '#{window_name}')"
target_auto="$(tmux display-message -p -t "$target_window" '#{automatic-rename}')"

sidebar="$(tmux show-option -qv -t "$session" @agent_watch_sidebar_pane 2>/dev/null || true)"
if [ -n "$sidebar" ] && ! tmux display-message -p -t "$sidebar" '#{pane_id}' >/dev/null 2>&1; then
  sidebar=''
  tmux set-option -q -t "$session" @agent_watch_sidebar_pane ''
fi

expanded="$(tmux show-option -qv -t "$session" @agent_watch_sidebar_expanded 2>/dev/null || true)"
if [ "$expanded" = on ]; then
  width="$(tmux_option @agent-watch-sidebar-expanded-width 38)"
else
  width="$(tmux_option @agent-watch-sidebar-width 3)"
fi

if [ -n "$sidebar" ]; then
  sidebar_window="$(tmux display-message -p -t "$sidebar" '#{window_id}')"
  if [ "$sidebar_window" = "$target_window" ]; then
    sidebar_left="$(tmux display-message -p -t "$sidebar" '#{pane_left}')"
    if [ "$sidebar_left" != 0 ]; then
      leftmost="$(tmux list-panes -t "$target_window" -F '#{pane_id}|#{pane_left}' |
        awk -F '|' '$2 == 0 { print $1; exit }')"
      [ -n "$leftmost" ] && tmux swap-pane -d -s "$sidebar" -t "$leftmost"
      tmux resize-pane -t "$sidebar" -x "$width" 2>/dev/null || true
    fi
    exit 0
  fi
  [ "$target_pane" = "$sidebar" ] && exit 0
  source_name="$(tmux display-message -p -t "$sidebar_window" '#{window_name}')"
  source_auto="$(tmux display-message -p -t "$sidebar_window" '#{automatic-rename}')"
  tmux join-pane -d -b -h -l "$width" -s "$sidebar" -t "$target_pane" 2>/dev/null || true
  tmux rename-window -t "$sidebar_window" "$source_name" 2>/dev/null || true
  tmux set-window-option -q -t "$sidebar_window" automatic-rename "$source_auto" 2>/dev/null || true
  tmux rename-window -t "$target_window" "$target_name" 2>/dev/null || true
  tmux set-window-option -q -t "$target_window" automatic-rename "$target_auto" 2>/dev/null || true
  exit 0
fi

sidebar="$(tmux split-window -d -b -h -l "$width" -t "$target_pane" -P -F '#{pane_id}' \
  "$PLUGIN_DIR/scripts/sidebar-render.sh '$session'")" || exit 0
tmux set-option -pq -t "$sidebar" @agent_watch_sidebar 1
tmux set-option -q -t "$session" @agent_watch_sidebar_pane "$sidebar"
tmux select-pane -t "$target_pane"
tmux rename-window -t "$target_window" "$target_name"
tmux set-window-option -q -t "$target_window" automatic-rename "$target_auto"
