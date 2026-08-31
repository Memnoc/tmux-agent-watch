#!/usr/bin/env bash

set -eu

ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
SOCKET="agent-watch-test-$$"
TMP_DIR="$(mktemp -d)"

cleanup() {
  tmux -L "$SOCKET" kill-server 2>/dev/null || true
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

ln -s "$(command -v sleep)" "$TMP_DIR/codex"
tmux -L "$SOCKET" -f /dev/null new-session -d -s agents "$TMP_DIR/codex 30"
socket_path="$(tmux -L "$SOCKET" display-message -p '#{socket_path}')"
server_pid="$(tmux -L "$SOCKET" display-message -p '#{pid}')"
tmux -L "$SOCKET" set-environment -g TMUX "$socket_path,$server_pid,0"
tmux -L "$SOCKET" run-shell "$ROOT/tmux-agent-watch.tmux"
sleep 1

format="$(tmux -L "$SOCKET" show-option -gqv window-status-format)"
case "$format" in
  *'#{@agent_watch_marker}'*) printf 'ok: window marker installed\n' ;;
  *) printf 'not ok: window marker missing\n'; exit 1 ;;
esac

binding="$(tmux -L "$SOCKET" list-keys -T prefix | grep 'scripts/next-attention.sh' || true)"
[ -n "$binding" ] || { printf 'not ok: attention binding missing\n'; exit 1; }
printf 'ok: attention binding installed\n'

state="$(tmux -L "$SOCKET" show-option -wqv -t agents:0 @agent_watch_state)"
[ "$state" = working ] || { printf 'not ok: expected working, got %s\n' "$state"; exit 1; }
printf 'ok: observer classified agent\n'
