#!/usr/bin/env bash

set -eu
source "$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)/lib.sh"

usage() {
  printf 'usage: %s working|needs_input|done|failed [message]\n' "$0" >&2
  exit 2
}

state="${1:-}"
message="${2:-}"
case "$state" in
  working|needs_input|done|failed) ;;
  *) usage ;;
esac

window_id="${TMUX_PANE:+$(tmux display-message -p -t "$TMUX_PANE" '#{window_id}')}"
[ -n "$window_id" ] || { printf 'agent-watch: not inside tmux\n' >&2; exit 1; }
set_window_state "$window_id" "$state" "$message" hook
