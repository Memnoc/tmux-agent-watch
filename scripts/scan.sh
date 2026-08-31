#!/usr/bin/env bash

set -u
source "$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)/lib.sh"

declare -A agent_windows=()

while IFS='|' read -r window_id pane_id command dead; do
  [ -n "$window_id" ] || continue

  if [ "$dead" = 1 ] && is_agent_command "$command"; then
    agent_windows["$window_id"]=1
    set_window_state "$window_id" failed 'process exited'
    continue
  fi

  if ! is_agent_command "$command"; then
    continue
  fi

  agent_windows["$window_id"]=1
  output="$(tmux capture-pane -p -t "$pane_id" -S -80 2>/dev/null || true)"
  result="$(classify_output "$command" "$output")"
  state="${result%%$'\t'*}"
  message="${result#*$'\t'}"
  set_window_state "$window_id" "$state" "$message"
done < <(tmux list-panes -a -F '#{window_id}|#{pane_id}|#{pane_current_command}|#{pane_dead}' 2>/dev/null)

while IFS= read -r window_id; do
  [ -n "${agent_windows[$window_id]:-}" ] || set_window_state "$window_id" unmanaged
done < <(tmux list-windows -a -F '#{window_id}' 2>/dev/null)
