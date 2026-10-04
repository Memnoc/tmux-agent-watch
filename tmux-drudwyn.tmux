#!/usr/bin/env bash

set -eu

PLUGIN_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"
PREVIOUS_PLUGIN_DIR="$(tmux show-option -gqv @drudwyn_plugin_dir 2>/dev/null || true)"
tmux set-option -gq @drudwyn_plugin_dir "$PLUGIN_DIR"

bash "$PLUGIN_DIR/scripts/migrate-options.sh"

option() {
  local name="$1" default="$2" value
  value="$(tmux show-option -gqv "$name")"
  printf '%s' "${value:-$default}"
}

install_window_formats() {
  local format current style_format
  style_format='#{@drudwyn_window_style}'
  for format in window-status-format window-status-current-format; do
    current="$(tmux show-option -gqv "$format")"
    current="${current//'#{@drudwyn_window_style}#I#[default]'/#I}"
    current="${current//'#{@agent_watch_marker}'/}"
    current="${current//'#{@agent_watch_window_style}'/}"
    current="${current//'#{@drudwyn_marker}'/}"
    current="${current//'#{@drudwyn_window_style}'/}"
    if [ "$(option @drudwyn-status off)" = on ]; then
      current="#{@drudwyn_marker}${current}"
    fi
    if [ "$(option @drudwyn-color-window-names on)" = on ]; then
      current="${current//#I/${style_format}#I#[default]}"
    fi
    tmux set-option -gq "$format" "$current"
  done
}

cleanup_plugin_hooks() {
  local hook slot command
  for hook in client-attached after-new-window after-select-pane after-select-window; do
    while read -r slot command; do
      if [[ "$command" == *"$PLUGIN_DIR"* ]] ||
        { [ -n "$PREVIOUS_PLUGIN_DIR" ] && [[ "$command" == *"$PREVIOUS_PLUGIN_DIR"* ]]; }; then
        tmux set-hook -gu "$slot"
      fi
    done < <(tmux show-hooks -g "$hook" 2>/dev/null || true)
  done
}

cleanup_sidebars() {
  local session sidebar
  while IFS= read -r session; do
    [ -n "$session" ] || continue
    sidebar="$(tmux show-option -qv -t "$session" @drudwyn_sidebar_pane 2>/dev/null || true)"
    if [ -n "$sidebar" ] &&
      [ "$(tmux show-option -pqv -t "$sidebar" @drudwyn_sidebar 2>/dev/null || true)" = 1 ]; then
      tmux kill-pane -t "$sidebar" 2>/dev/null || true
    fi
    tmux set-option -q -t "$session" @drudwyn_sidebar_pane ''
    tmux set-option -q -t "$session" @drudwyn_sidebar_expanded off
  done < <(tmux list-sessions -F '#{session_name}' 2>/dev/null || true)
}

cleanup_plugin_hooks
# A runtime-location migration also moves the observer. Only stop the recorded
# PID when its command still belongs to the previous Drudwyn installation.
if [ -n "$PREVIOUS_PLUGIN_DIR" ] && [ "$PREVIOUS_PLUGIN_DIR" != "$PLUGIN_DIR" ]; then
  previous_watcher="$(tmux show-option -gqv @drudwyn_watcher_pid 2>/dev/null || true)"
  if [ -n "$previous_watcher" ]; then
    case "$(ps -p "$previous_watcher" -o args= 2>/dev/null || true)" in
      *"$PREVIOUS_PLUGIN_DIR/scripts/start-watcher.sh"*)
        kill -TERM "$previous_watcher" 2>/dev/null || true
        tmux set-option -gu @drudwyn_watcher_pid
        ;;
    esac
  fi
fi
install_window_formats
if [ "$(option @drudwyn-sidebar off)" != on ]; then
  cleanup_sidebars
fi
if [ "$(option @drudwyn-hud on)" = on ]; then
  "$PLUGIN_DIR/scripts/hud-install.sh"
else
  bash "$PLUGIN_DIR/scripts/hud-install.sh" --disable
fi

tmux bind-key "$(option @drudwyn-next-key a)" run-shell "$PLUGIN_DIR/scripts/next-attention.sh #{q:client_name}"
tmux bind-key "$(option @drudwyn-sidebar-key Space)" run-shell "$PLUGIN_DIR/scripts/sidebar-resize.sh"
tmux bind-key "$(option @drudwyn-restart-key A)" run-shell "$PLUGIN_DIR/scripts/sidebar-restart.sh"
tmux bind-key "$(option @drudwyn-finish-key X)" display-popup -EE -w 70% -h 30% \
  -d '#{pane_current_path}' "$PLUGIN_DIR/scripts/worktree-finish.sh"
tmux bind-key "$(option @drudwyn-help-key H)" display-popup -E -w 72 -h 24 \
  "$PLUGIN_DIR/scripts/help.sh"
tmux bind-key "$(option @drudwyn-options-key O)" display-popup -EE -w 96 -h 30 \
  "$PLUGIN_DIR/scripts/settings.sh"
if [ "$(option @drudwyn-v2 on)" = on ]; then
  coordinator_key="$(option @drudwyn-coordinator-key G)"
  coordinator_binding="$(tmux list-keys -T prefix "$coordinator_key" 2>/dev/null || true)"
  case "$coordinator_binding" in
    ''|*'/scripts/coordinator-toggle.sh '*) tmux bind-key "$coordinator_key" run-shell "$PLUGIN_DIR/scripts/coordinator-toggle.sh #{q:client_name}" ;;
  esac
  tmux bind-key "$(option @drudwyn-worktree-key W)" run-shell \
    "$PLUGIN_DIR/scripts/navigation-popup.sh #{q:client_name} cockpit --start"
  tmux bind-key "$(option @drudwyn-cockpit-key P)" run-shell \
    "$PLUGIN_DIR/scripts/navigation-popup.sh #{q:client_name} cockpit"
  status_key="$(option @drudwyn-status-key g)"
  status_binding="$(tmux list-keys -T prefix "$status_key" 2>/dev/null || true)"
  case "$status_binding" in
    ''|*'switch-client -T drudwyn-status'*) tmux bind-key "$status_key" switch-client -T drudwyn-status ;;
  esac
  for mapping in f:failed i:input r:review a:attention w:windows; do
    tmux bind-key -T drudwyn-status "${mapping%%:*}" run-shell \
      "$PLUGIN_DIR/scripts/v2.sh status-action ${mapping#*:} --client #{q:client_name}"
  done
  tmux bind-key -T drudwyn-status Escape switch-client -T root
else
  status_key="$(option @drudwyn-status-key g)"
  if tmux list-keys -T prefix "$status_key" 2>/dev/null | grep -Fq 'switch-client -T drudwyn-status'; then
    tmux unbind-key "$status_key"
  fi
  tmux bind-key "$(option @drudwyn-worktree-key W)" command-prompt -p 'Branch:' \
    "run-shell '$PLUGIN_DIR/scripts/worktree-new.sh --repo \"#{pane_current_path}\" \"%%\"'"
  tmux bind-key "$(option @drudwyn-cockpit-key P)" display-popup -EE -w 78 -h 26 \
    -d '#{pane_current_path}' "$PLUGIN_DIR/scripts/cockpit.sh"
fi
tmux bind-key "$(option @drudwyn-navigator-key w)" run-shell \
    "$PLUGIN_DIR/scripts/navigation-popup.sh #{q:client_name} navigator"
tmux bind-key "$(option @drudwyn-native-navigator-key C-w)" choose-tree -Zw
tmux bind-key "$(option @drudwyn-session-key s)" run-shell \
    "$PLUGIN_DIR/scripts/navigation-popup.sh #{q:client_name} sessions"
tmux bind-key "$(option @drudwyn-native-session-key S)" choose-tree -Zs
tmux bind-key '{' run-shell "$PLUGIN_DIR/scripts/safe-swap.sh -U"
tmux bind-key '}' run-shell "$PLUGIN_DIR/scripts/safe-swap.sh -D"
tmux bind-key -n MouseDown1Pane if-shell -F '#{==:#{@drudwyn_sidebar},1}' \
  "run-shell '$PLUGIN_DIR/scripts/sidebar-click.sh #{pane_id} #{mouse_y} #{q:client_name}'" \
  'select-pane -t ='
tmux bind-key -n MouseDown1Status if-shell -F '#{mouse_status_range}' \
  "run-shell '$PLUGIN_DIR/scripts/status-click.sh #{q:mouse_status_range} #{q:client_name}'" \
  'if-shell -F "#{&&:#{!=:#{@drudwyn-v2},off},#{!=:#{@drudwyn-hud},off}}" "run-shell true" "select-window -t ="'
tmux bind-key -n WheelUpPane if-shell -F '#{==:#{@drudwyn_sidebar},1}' \
  'run-shell ":"' \
  'if-shell -F "#{||:#{pane_in_mode},#{mouse_any_flag}}" "send-keys -M" "copy-mode -e"'
tmux set-hook -g 'client-attached[100]' "run-shell -b '$PLUGIN_DIR/scripts/start-watcher.sh'"
if [ "$(option @drudwyn-v2 on)" = on ]; then
  tmux set-hook -g 'after-new-window[100]' "run-shell -b '$PLUGIN_DIR/scripts/v2.sh scan'"
else
  tmux set-hook -g 'after-new-window[100]' "run-shell -b '$PLUGIN_DIR/scripts/scan.sh'"
fi
tmux set-hook -g 'after-select-pane[100]' \
  "if-shell -F '#{==:#{@drudwyn_sidebar},1}' 'select-pane -l'"
tmux set-hook -g 'after-select-window[100]' "run-shell -b '$PLUGIN_DIR/scripts/sidebar-ensure.sh #{pane_id}'"

"$PLUGIN_DIR/scripts/start-watcher.sh"
