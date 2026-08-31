#!/usr/bin/env bash

set -u
source "$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)/lib.sh"

target="${TMUX_PANE:-$(tmux display-message -p '#{pane_id}')}"
session="$(tmux display-message -p -t "$target" '#{session_name}')"
sidebar="$(tmux show-option -qv -t "$session" @agent_watch_sidebar_pane 2>/dev/null || true)"
if [ -z "$sidebar" ] || ! tmux display-message -p -t "$sidebar" '#{pane_id}' >/dev/null 2>&1; then
  tmux set-option -q -t "$session" @agent_watch_sidebar_pane ''
  "$PLUGIN_DIR/scripts/sidebar-ensure.sh" "$target"
  sidebar="$(tmux show-option -qv -t "$session" @agent_watch_sidebar_pane 2>/dev/null || true)"
  [ -n "$sidebar" ] || exit 1
fi

expanded="$(tmux show-option -qv -t "$session" @agent_watch_sidebar_expanded 2>/dev/null || true)"
if [ "$expanded" = on ]; then
  tmux set-option -q -t "$session" @agent_watch_sidebar_expanded off
  width="$(tmux_option @agent-watch-sidebar-width 3)"
else
  tmux set-option -q -t "$session" @agent_watch_sidebar_expanded on
  width="$(tmux_option @agent-watch-sidebar-expanded-width 38)"
fi
if ! tmux resize-pane -t "$sidebar" -x "$width" 2>/dev/null; then
  tmux set-option -q -t "$session" @agent_watch_sidebar_pane ''
  "$PLUGIN_DIR/scripts/sidebar-ensure.sh" "$target"
  sidebar="$(tmux show-option -qv -t "$session" @agent_watch_sidebar_pane 2>/dev/null || true)"
  [ -n "$sidebar" ] || exit 1
  tmux resize-pane -t "$sidebar" -x "$width"
fi
