#!/usr/bin/env bash

set -eu
source "$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)/lib.sh"

target="${TMUX_PANE:-$(tmux display-message -p '#{pane_id}')}"
session="$(tmux display-message -p -t "$target" '#{session_name}')"
window_id="$(tmux display-message -p -t "$target" '#{window_id}')"

if [ "$(tmux show-option -pqv -t "$target" @agent_watch_sidebar 2>/dev/null || true)" = 1 ]; then
  target="$(tmux list-panes -t "$window_id" -F '#{pane_id}|#{@agent_watch_sidebar}' |
    awk -F '|' '$2 != 1 { print $1; exit }')"
  [ -n "$target" ] || exit 1
fi

sidebar="$(tmux show-option -qv -t "$session" @agent_watch_sidebar_pane 2>/dev/null || true)"
tmux set-option -q -t "$session" @agent_watch_sidebar_pane ''

if [ -n "$sidebar" ] &&
  [ "$(tmux show-option -pqv -t "$sidebar" @agent_watch_sidebar 2>/dev/null || true)" = 1 ]; then
  tmux kill-pane -t "$sidebar"
fi

"$PLUGIN_DIR/scripts/sidebar-ensure.sh" "$target"
sidebar="$(tmux show-option -qv -t "$session" @agent_watch_sidebar_pane 2>/dev/null || true)"
[ -n "$sidebar" ] || exit 1
tmux display-message 'Agent sidebar restarted'
