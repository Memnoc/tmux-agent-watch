#!/usr/bin/env bash

set -eu
source "$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)/lib.sh"

event="${1:-}"
if [ "$(tmux_option @agent-watch-v2 on)" = on ]; then
  exec "$PLUGIN_DIR/scripts/v2.sh" hook open-code "$event"
fi
message="${2:-}"
window_id="${TMUX_PANE:+$(tmux display-message -p -t "$TMUX_PANE" '#{window_id}')}"
[ -n "$window_id" ] || exit 0

pane_summary() {
  local output result summary
  output="$(tmux capture-pane -p -t "$TMUX_PANE" -S -200 2>/dev/null || true)"
  result="$(classify_output opencode "$output")"
  summary="${result#*$'\t'}"
  case "$summary" in
    ''|Working|'Ready for review') return ;;
    *) printf '%s' "$summary" ;;
  esac
}

case "$event" in
  working)
    set_window_state "$window_id" working "${message:-Working}" hook
    ;;
  permission)
    set_window_state "$window_id" needs_input "${message:-Approval required}" hook
    ;;
  idle)
    summary="$(pane_summary)"
    set_window_state "$window_id" done "${summary:-Ready for review}" hook
    ;;
  error)
    set_window_state "$window_id" failed "${message:-Agent stopped with an error}" hook
    ;;
  *) exit 2 ;;
esac
