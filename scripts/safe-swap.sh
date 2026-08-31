#!/usr/bin/env bash

set -eu

direction="${1:-}"
case "$direction" in -U|-D) ;; *) exit 2 ;; esac

session="$(tmux display-message -p '#{session_name}')"
window_id="$(tmux display-message -p '#{window_id}')"
sidebar="$(tmux show-option -qv -t "$session" @agent_watch_sidebar_pane 2>/dev/null || true)"

if [ -n "$sidebar" ] &&
  [ "$(tmux display-message -p -t "$sidebar" '#{window_id}' 2>/dev/null || true)" = "$window_id" ]; then
  tmux display-message 'Agent sidebar position is fixed'
  exit 0
fi

tmux swap-pane "$direction"
