#!/usr/bin/env bash

set -u
source "$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)/lib.sh"

declare -A agent_windows=()

while IFS='|' read -r window_id pane_id command dead path; do
  [ -n "$window_id" ] || continue

  if [ "$dead" = 1 ] && is_agent_command "$command"; then
    agent_windows["$window_id"]=1
    set_window_state "$window_id" failed 'process exited' process
    continue
  fi

  if ! is_agent_command "$command"; then
    continue
  fi

  agent_windows["$window_id"]=1
  set_window_git_context "$window_id" "$path"
  output="$(tmux capture-pane -p -t "$pane_id" -S -200 2>/dev/null || true)"
  result="$(classify_output "$command" "$output")"
  state="${result%%$'\t'*}"
  message="${result#*$'\t'}"
  set_window_state "$window_id" "$state" "$message"
done < <(tmux list-panes -a -F '#{window_id}|#{pane_id}|#{pane_current_command}|#{pane_dead}|#{pane_current_path}' 2>/dev/null)

while IFS= read -r window_id; do
  [ -n "${agent_windows[$window_id]:-}" ] || set_window_state "$window_id" unmanaged
done < <(tmux list-windows -a -F '#{window_id}' 2>/dev/null)

if [ "$(tmux_option @agent-watch-sidebar off)" = on ]; then
  while IFS= read -r session; do
    [ -n "$session" ] && "$PLUGIN_DIR/scripts/sidebar-ensure.sh" "$session:"
  done < <(tmux list-windows -a -F '#{session_name}|#{@agent_watch_state}' 2>/dev/null |
    awk -F '|' '$2 != "" && !seen[$1]++ { print $1 }')
fi
