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
tmux -L "$SOCKET" set-option -g @agent-watch-v2 off
tmux -L "$SOCKET" set-option -g @agent-watch-sidebar on
socket_path="$(tmux -L "$SOCKET" display-message -p '#{socket_path}')"
server_pid="$(tmux -L "$SOCKET" display-message -p '#{pid}')"
tmux -L "$SOCKET" set-environment -g TMUX "$socket_path,$server_pid,0"
tmux -L "$SOCKET" run-shell "$ROOT/tmux-agent-watch.tmux"
sleep 1

hud_fleet="$(tmux -L "$SOCKET" show-option -gqv 'status-format[0]')$(tmux -L "$SOCKET" show-option -gqv 'status-format[1]')"
case "$hud_fleet" in *'scripts/status-bar.sh'*) ;; *) printf 'not ok: clustered status bar is missing\n'; exit 1 ;; esac
case "$hud_fleet" in *'scripts/status-separator.sh'*) ;; *) printf 'not ok: terminal separator is missing\n'; exit 1 ;; esac
printf 'ok: clustered bar and terminal separator are installed\n'

navigator_binding="$(tmux -L "$SOCKET" list-keys -T prefix | awk '$4 == "w" && /scripts\/v2.sh navigator/')"
native_binding="$(tmux -L "$SOCKET" list-keys -T prefix | awk '$4 == "C-w" && /choose-tree -Zw/')"
[ -n "$navigator_binding" ] && [ -n "$native_binding" ] || {
  printf 'not ok: grouped and native navigator bindings are not both available\n'; exit 1;
}
printf 'ok: grouped navigation preserves the native chooser fallback\n'

session_binding="$(tmux -L "$SOCKET" list-keys -T prefix | awk '$4 == "s" && /scripts\/v2.sh sessions/')"
native_session_binding="$(tmux -L "$SOCKET" list-keys -T prefix | awk '$4 == "S" && /choose-tree -Zs/')"
[ -n "$session_binding" ] && [ -n "$native_session_binding" ] || {
  printf 'not ok: compact and native session navigator bindings are not both available\n'; exit 1;
}
printf 'ok: session navigation preserves the native chooser fallback\n'

binding="$(tmux -L "$SOCKET" list-keys -T prefix | grep 'scripts/next-attention.sh' || true)"
[ -n "$binding" ] || { printf 'not ok: attention binding missing\n'; exit 1; }
printf 'ok: attention binding installed\n'

sidebar_binding="$(tmux -L "$SOCKET" list-keys -T prefix | grep 'Space.*scripts/sidebar-resize.sh' || true)"
[ -n "$sidebar_binding" ] || { printf 'not ok: sidebar binding missing\n'; exit 1; }
printf 'ok: sidebar binding installed\n'

restart_binding="$(tmux -L "$SOCKET" list-keys -T prefix | grep ' A .*scripts/sidebar-restart.sh' || true)"
[ -n "$restart_binding" ] || { printf 'not ok: sidebar restart binding missing\n'; exit 1; }
printf 'ok: sidebar restart binding installed\n'

worktree_binding="$(tmux -L "$SOCKET" list-keys -T prefix | grep ' W .*scripts/worktree-new.sh' || true)"
[ -n "$worktree_binding" ] || { printf 'not ok: worktree launcher binding missing\n'; exit 1; }
printf 'ok: worktree launcher binding installed\n'

finish_binding="$(tmux -L "$SOCKET" list-keys -T prefix | grep ' X .*scripts/worktree-finish.sh' || true)"
[ -n "$finish_binding" ] || { printf 'not ok: guided worktree finish binding missing\n'; exit 1; }
printf 'ok: guided worktree finish binding installed\n'

if tmux -L "$SOCKET" list-keys -T prefix | grep -q 'scripts/worktree-lazygit.sh'; then
  printf 'not ok: removed lazygit binding is still installed\n'; exit 1
fi
printf 'ok: no external Git UI binding is installed\n'

sidebar_wheel_binding="$(tmux -L "$SOCKET" list-keys -T root | grep 'WheelUpPane.*@agent_watch_sidebar' || true)"
[ -n "$sidebar_wheel_binding" ] || { printf 'not ok: sidebar wheel guard missing\n'; exit 1; }
printf 'ok: sidebar blocks wheel scrolling without changing other panes\n'

swap_bindings="$(tmux -L "$SOCKET" list-keys -T prefix | grep -c 'scripts/safe-swap.sh')"
[ "$swap_bindings" = 2 ] || { printf 'not ok: guarded pane swap bindings missing\n'; exit 1; }
printf 'ok: pane swap bindings protect sidebar position\n'

tmux -L "$SOCKET" run-shell "$ROOT/tmux-agent-watch.tmux"
tmux -L "$SOCKET" run-shell "$ROOT/tmux-agent-watch.tmux"
hook_count="$(tmux -L "$SOCKET" show-hooks -g after-new-window | grep -c "$ROOT/scripts/scan.sh")"
[ "$hook_count" = 1 ] || { printf 'not ok: plugin reload duplicated hooks\n'; exit 1; }
printf 'ok: plugin reload keeps hooks unique\n'

state="$(tmux -L "$SOCKET" show-option -wqv -t agents:0 @agent_watch_state)"
[ "$state" = working ] || { printf 'not ok: expected working, got %s\n' "$state"; exit 1; }
printf 'ok: observer classified agent\n'

first_pane="$(tmux -L "$SOCKET" list-panes -t agents:0 -F '#{pane_id}|#{@agent_watch_sidebar}' |
  awk -F '|' '$2 != 1 { print $1; exit }')"
printf '{"prompt":"implement exact lifecycle states"}' |
  TMUX="$socket_path,$server_pid,0" TMUX_PANE="$first_pane" "$ROOT/scripts/claude-hook.sh" UserPromptSubmit
hook_message="$(tmux -L "$SOCKET" show-option -wqv -t agents:0 @agent_watch_message)"
hook_source="$(tmux -L "$SOCKET" show-option -wqv -t agents:0 @agent_watch_source)"
[ "$hook_message" = 'implement exact lifecycle states' ] && [ "$hook_source" = hook ] || {
  printf 'not ok: lifecycle hook did not publish an exact state\n'; exit 1;
}
printf '{}' | TMUX="$socket_path,$server_pid,0" TMUX_PANE="$first_pane" "$ROOT/scripts/claude-hook.sh" Stop
sleep 2
state="$(tmux -L "$SOCKET" show-option -wqv -t agents:0 @agent_watch_state)"
[ "$state" = done ] || { printf 'not ok: observer overwrote exact hook state with %s\n' "$state"; exit 1; }
printf 'ok: exact lifecycle state outranks observer fallback\n'

printf '{"prompt":"verify Codex hook events"}' |
  TMUX="$socket_path,$server_pid,0" TMUX_PANE="$first_pane" "$ROOT/scripts/codex-hook.sh" userPromptSubmit
hook_message="$(tmux -L "$SOCKET" show-option -wqv -t agents:0 @agent_watch_message)"
[ "$hook_message" = 'verify Codex hook events' ] || {
  printf 'not ok: Codex lifecycle hook did not capture its prompt\n'; exit 1;
}
printf '{}' | TMUX="$socket_path,$server_pid,0" TMUX_PANE="$first_pane" "$ROOT/scripts/codex-hook.sh" permissionRequest
state="$(tmux -L "$SOCKET" show-option -wqv -t agents:0 @agent_watch_state)"
[ "$state" = needs_input ] || { printf 'not ok: Codex permission hook produced %s\n' "$state"; exit 1; }
printf 'ok: Codex lifecycle events publish exact states\n'

TMUX="$socket_path,$server_pid,0" TMUX_PANE="$first_pane" "$ROOT/scripts/opencode-hook.sh" working
TMUX="$socket_path,$server_pid,0" TMUX_PANE="$first_pane" "$ROOT/scripts/opencode-hook.sh" permission 'Approve file write'
message="$(tmux -L "$SOCKET" show-option -wqv -t agents:0 @agent_watch_message)"
[ "$message" = 'Approve file write' ] || {
  printf 'not ok: OpenCode lifecycle hook lost its permission reason\n'; exit 1;
}
TMUX="$socket_path,$server_pid,0" TMUX_PANE="$first_pane" "$ROOT/scripts/opencode-hook.sh" idle
state="$(tmux -L "$SOCKET" show-option -wqv -t agents:0 @agent_watch_state)"
[ "$state" = done ] || { printf 'not ok: OpenCode idle hook produced %s\n' "$state"; exit 1; }
printf 'ok: OpenCode lifecycle events publish exact states\n'

fleet_output="$(TMUX="$socket_path,$server_pid,0" "$ROOT/scripts/hud.sh" fleet agents @0)"
selected_output="$(TMUX="$socket_path,$server_pid,0" "$ROOT/scripts/hud.sh" selected agents @0)"
printf '%s' "$fleet_output" | grep -Fq '1 agents' || { printf 'not ok: HUD fleet count missing\n'; exit 1; }
printf '%s' "$selected_output" | grep -Fq 'REVIEW' || { printf 'not ok: HUD selected state missing\n'; exit 1; }
printf 'ok: HUD renders fleet and selected agent\n'

sidebar="$(tmux -L "$SOCKET" show-option -qv -t agents @agent_watch_sidebar_pane)"
[ -n "$sidebar" ] || { printf 'not ok: sidebar was not created\n'; exit 1; }
sidebar_marker="$(tmux -L "$SOCKET" show-option -pqv -t "$sidebar" @agent_watch_sidebar)"
[ "$sidebar_marker" = 1 ] || { printf 'not ok: sidebar pane is not marked\n'; exit 1; }
sidebar_width="$(tmux -L "$SOCKET" display-message -p -t "$sidebar" '#{pane_width}')"
[ "$sidebar_width" = 3 ] || { printf 'not ok: collapsed sidebar width is %s\n' "$sidebar_width"; exit 1; }
agent_name="$(tmux -L "$SOCKET" display-message -p -t agents:0 '#{window_name}')"
[ "$agent_name" = codex ] || { printf 'not ok: sidebar changed agent window name to %s\n' "$agent_name"; exit 1; }
printf 'ok: sidebar created for agent session\n'

old_sidebar="$sidebar"
TMUX="$socket_path,$server_pid,0" TMUX_PANE="$first_pane" "$ROOT/scripts/sidebar-restart.sh"
sidebar="$(tmux -L "$SOCKET" show-option -qv -t agents @agent_watch_sidebar_pane)"
[ "$sidebar" != "$old_sidebar" ] &&
  [ "$(tmux -L "$SOCKET" show-option -pqv -t "$sidebar" @agent_watch_sidebar)" = 1 ] || {
  printf 'not ok: sidebar restart did not replace the generated pane\n'; exit 1;
}
printf 'ok: sidebar restart replaces only the generated pane\n'

tmux -L "$SOCKET" kill-pane -t "$sidebar"
TMUX="$socket_path,$server_pid,0" TMUX_PANE="$first_pane" "$ROOT/scripts/sidebar-resize.sh"
sidebar="$(tmux -L "$SOCKET" show-option -qv -t agents @agent_watch_sidebar_pane)"
tmux -L "$SOCKET" display-message -p -t "$sidebar" '#{pane_id}' >/dev/null 2>&1 || {
  printf 'not ok: sidebar toggle did not recover a stale pane ID\n'; exit 1;
}
expanded="$(tmux -L "$SOCKET" show-option -qv -t agents @agent_watch_sidebar_expanded)"
[ "$expanded" = on ] || {
  printf 'not ok: recovered sidebar did not honor the requested expansion\n'; exit 1;
}
printf 'ok: sidebar toggle recovers stale pane metadata\n'
TMUX="$socket_path,$server_pid,0" TMUX_PANE="$first_pane" "$ROOT/scripts/sidebar-resize.sh"

tmux -L "$SOCKET" select-pane -t "$first_pane"
tmux -L "$SOCKET" select-pane -L
focused_pane="$(tmux -L "$SOCKET" display-message -p '#{pane_id}')"
[ "$focused_pane" != "$sidebar" ] || {
  printf 'not ok: sidebar accepted focus from normal pane navigation\n'; exit 1;
}
printf 'ok: sidebar rejects normal pane focus\n'

tmux -L "$SOCKET" swap-pane -d -s "$sidebar" -t "$first_pane"
sleep 3
sidebar_left="$(tmux -L "$SOCKET" display-message -p -t "$sidebar" '#{pane_left}')"
[ "$sidebar_left" = 0 ] || {
  printf 'not ok: sidebar did not recover its reserved left position\n'; exit 1;
}
printf 'ok: sidebar recovers its position after pane swaps\n'

tmux -L "$SOCKET" new-window -d -t agents -n second "$TMP_DIR/codex 30"
second_pane="$(tmux -L "$SOCKET" list-panes -t agents:second -F '#{pane_id}' | head -n 1)"
tmux -L "$SOCKET" select-window -t agents:second
sleep 1
sidebar_window="$(tmux -L "$SOCKET" display-message -p -t "$sidebar" '#{window_id}')"
second_window="$(tmux -L "$SOCKET" display-message -p -t "$second_pane" '#{window_id}')"
[ "$sidebar_window" = "$second_window" ] || { printf 'not ok: sidebar did not follow target window\n'; exit 1; }
second_name="$(tmux -L "$SOCKET" display-message -p -t "$second_window" '#{window_name}')"
[ "$second_name" = second ] || { printf 'not ok: sidebar changed destination window name\n'; exit 1; }
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

sleep 1
TMUX="$socket_path,$server_pid,0" TMUX_PANE="$second_pane" "$ROOT/scripts/sidebar-resize.sh"
sleep 0.2
collapsed_frame="$(tmux -L "$SOCKET" capture-pane -p -t "$sidebar")"
if printf '%s' "$collapsed_frame" | grep -q '[[:alpha:]]'; then
  printf 'not ok: stale expanded text remains after sidebar collapse\n'; exit 1
fi
printf 'ok: sidebar collapse redraws within 200ms\n'

tmux -L "$SOCKET" new-window -d -t agents -n shell
tmux -L "$SOCKET" select-window -t agents:shell
sleep 1
shell_window="$(tmux -L "$SOCKET" display-message -p -t agents:shell '#{window_id}')"
sidebar_window="$(tmux -L "$SOCKET" display-message -p -t "$sidebar" '#{window_id}')"
[ "$sidebar_window" != "$shell_window" ] || { printf 'not ok: sidebar invaded a new shell window\n'; exit 1; }
shell_panes="$(tmux -L "$SOCKET" display-message -p -t agents:shell '#{window_panes}')"
[ "$shell_panes" = 1 ] || { printf 'not ok: new shell window has %s panes\n' "$shell_panes"; exit 1; }
printf 'ok: new shell windows remain full-width until an agent starts\n'
