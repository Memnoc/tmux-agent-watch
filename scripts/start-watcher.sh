#!/usr/bin/env bash

set -u
source "$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)/lib.sh"

current_pid="$(tmux show-option -gqv @agent_watch_watcher_pid 2>/dev/null || true)"
if [ -n "$current_pid" ] && kill -0 "$current_pid" 2>/dev/null; then
  exit 0
fi

interval="$(tmux_option @agent-watch-interval 2)"
(
  trap 'exit 0' TERM INT
  while tmux list-sessions >/dev/null 2>&1; do
    "$PLUGIN_DIR/scripts/scan.sh"
    sleep "$interval"
  done
) >/dev/null 2>&1 &

tmux set-option -gq @agent_watch_watcher_pid "$!"

