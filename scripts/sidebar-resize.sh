#!/usr/bin/env bash

set -u
source "$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)/lib.sh"

target="${TMUX_PANE:-$(tmux display-message -p '#{pane_id}')}"
session="$(tmux display-message -p -t "$target" '#{session_name}')"
sidebar="$(tmux show-option -qv -t "$session" @agent_watch_sidebar_pane 2>/dev/null || true)"
[ -n "$sidebar" ] || { "$PLUGIN_DIR/scripts/sidebar-ensure.sh" "$target"; exit 0; }

expanded="$(tmux show-option -qv -t "$session" @agent_watch_sidebar_expanded 2>/dev/null || true)"
if [ "$expanded" = on ]; then
  tmux set-option -q -t "$session" @agent_watch_sidebar_expanded off
  width="$(tmux_option @agent-watch-sidebar-width 3)"
else
  tmux set-option -q -t "$session" @agent_watch_sidebar_expanded on
  width="$(tmux_option @agent-watch-sidebar-expanded-width 38)"
fi
tmux resize-pane -t "$sidebar" -x "$width"
