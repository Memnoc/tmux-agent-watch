#!/usr/bin/env bash

set -eu

PLUGIN_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"

option() {
  local name="$1" default="$2" value
  value="$(tmux show-option -gqv "$name")"
  printf '%s' "${value:-$default}"
}

install_window_formats() {
  local format current
  for format in window-status-format window-status-current-format; do
    current="$(tmux show-option -gqv "$format")"
    case "$current" in
      *'#{@agent_watch_marker}'*) ;;
      *) tmux set-option -gq "$format" "#{@agent_watch_marker}${current}" ;;
    esac
  done
}

install_window_formats

tmux bind-key "$(option @agent-watch-next-key a)" run-shell "$PLUGIN_DIR/scripts/next-attention.sh"
tmux bind-key "$(option @agent-watch-overview-key A)" display-popup -E -w 72 -h 70% "$PLUGIN_DIR/scripts/overview.sh"
tmux set-hook -ag client-attached "run-shell -b '$PLUGIN_DIR/scripts/start-watcher.sh'"
tmux set-hook -ag after-new-window "run-shell -b '$PLUGIN_DIR/scripts/scan.sh'"
tmux set-hook -ag pane-focus-in "run-shell -b '$PLUGIN_DIR/scripts/scan.sh'"

"$PLUGIN_DIR/scripts/start-watcher.sh"

