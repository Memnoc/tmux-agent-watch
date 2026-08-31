#!/usr/bin/env bash

set -eu
source "$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)/lib.sh"

event="${1:-}"
payload="$(cat)"
window_id="${TMUX_PANE:+$(tmux display-message -p -t "$TMUX_PANE" '#{window_id}')}"
[ -n "$window_id" ] || exit 0

json_value() {
  command -v jq >/dev/null 2>&1 || return
  printf '%s' "$payload" | jq -r "$1 // empty" 2>/dev/null || true
}

pane_summary() {
  local output result message
  output="$(tmux capture-pane -p -t "$TMUX_PANE" -S -200 2>/dev/null || true)"
  result="$(classify_output claude "$output")"
  message="${result#*$'\t'}"
  case "$message" in
    ''|Working|'Ready for review') return ;;
    *) printf '%s' "$message" ;;
  esac
}

case "$event" in
  UserPromptSubmit)
    set_window_state "$window_id" working "$(json_value '.prompt')" hook
    ;;
  PermissionRequest|permission_prompt)
    message="$(json_value '.message')"
    set_window_state "$window_id" needs_input "${message:-Approval required}" hook
    ;;
  Stop|idle_prompt)
    message="$(pane_summary)"
    set_window_state "$window_id" done "${message:-Ready for review}" hook
    ;;
  StopFailure)
    message="$(json_value '.error')"
    set_window_state "$window_id" failed "${message:-Agent stopped with an error}" hook
    ;;
  *) exit 2 ;;
esac
