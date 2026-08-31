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

sidebar_binding="$(tmux -L "$SOCKET" list-keys -T prefix | grep 'Space.*scripts/sidebar-resize.sh' || true)"
[ -n "$sidebar_binding" ] || { printf 'not ok: sidebar binding missing\n'; exit 1; }
printf 'ok: sidebar binding installed\n'

tmux -L "$SOCKET" run-shell "$ROOT/tmux-agent-watch.tmux"
tmux -L "$SOCKET" run-shell "$ROOT/tmux-agent-watch.tmux"
hook_count="$(tmux -L "$SOCKET" show-hooks -g after-new-window | grep -c "$ROOT/scripts/scan.sh")"
[ "$hook_count" = 1 ] || { printf 'not ok: plugin reload duplicated hooks\n'; exit 1; }
printf 'ok: plugin reload keeps hooks unique\n'

state="$(tmux -L "$SOCKET" show-option -wqv -t agents:0 @agent_watch_state)"
[ "$state" = working ] || { printf 'not ok: expected working, got %s\n' "$state"; exit 1; }
printf 'ok: observer classified agent\n'

sidebar="$(tmux -L "$SOCKET" show-option -qv -t agents @agent_watch_sidebar_pane)"
[ -n "$sidebar" ] || { printf 'not ok: sidebar was not created\n'; exit 1; }
sidebar_marker="$(tmux -L "$SOCKET" show-option -pqv -t "$sidebar" @agent_watch_sidebar)"
[ "$sidebar_marker" = 1 ] || { printf 'not ok: sidebar pane is not marked\n'; exit 1; }
printf 'ok: sidebar created for agent session\n'

tmux -L "$SOCKET" new-window -d -t agents -n second "$TMP_DIR/codex 30"
second_pane="$(tmux -L "$SOCKET" list-panes -t agents:second -F '#{pane_id}' | head -n 1)"
TMUX="$socket_path,$server_pid,0" "$ROOT/scripts/sidebar-ensure.sh" "$second_pane"
sidebar_window="$(tmux -L "$SOCKET" display-message -p -t "$sidebar" '#{window_id}')"
second_window="$(tmux -L "$SOCKET" display-message -p -t "$second_pane" '#{window_id}')"
[ "$sidebar_window" = "$second_window" ] || { printf 'not ok: sidebar did not follow target window\n'; exit 1; }
printf 'ok: sidebar follows selected window\n'

sleep 1
click_map="$(tmux -L "$SOCKET" show-option -pqv -t "$sidebar" @agent_watch_click_map)"
printf '%s' "$click_map" | grep -Fq "1=${second_window}" || {
  printf 'not ok: sidebar click map missing second window %s: %s\n' "$second_window" "$click_map"
  exit 1
}
pane_top="$(tmux -L "$SOCKET" display-message -p -t "$sidebar" '#{pane_top}')"
TMUX="$socket_path,$server_pid,0" "$ROOT/scripts/sidebar-click.sh" "$sidebar" "$((pane_top + 1))"
selected="$(tmux -L "$SOCKET" display-message -p -t agents: '#{window_id}')"
[ "$selected" = "$second_window" ] || { printf 'not ok: sidebar click did not select window\n'; exit 1; }
printf 'ok: sidebar rows select agent windows\n'

TMUX="$socket_path,$server_pid,0" TMUX_PANE="$second_pane" "$ROOT/scripts/sidebar-resize.sh"
expanded="$(tmux -L "$SOCKET" show-option -qv -t agents @agent_watch_sidebar_expanded)"
[ "$expanded" = on ] || { printf 'not ok: sidebar did not expand\n'; exit 1; }
printf 'ok: sidebar expands with one action\n'
