#!/usr/bin/env bash

set -eu

PLUGIN_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"

option() {
  local name="$1" default="$2" value
  value="$(tmux show-option -gqv "$name")"
  printf '%s' "${value:-$default}"
}

install_window_formats() {
  local format current style_format
  style_format='#{@agent_watch_window_style}'
  for format in window-status-format window-status-current-format; do
    current="$(tmux show-option -gqv "$format")"
    current="${current//'#{@agent_watch_marker}'/}"
    current="${current//'#{@agent_watch_window_style}'/}"
    if [ "$(option @agent-watch-status off)" = on ]; then
      current="#{@agent_watch_marker}${current}"
    fi
    if [ "$(option @agent-watch-color-window-names on)" = on ]; then
      current="${current//#I/${style_format}#I}"
    fi
    tmux set-option -gq "$format" "$current"
  done
}

cleanup_plugin_hooks() {
  local hook slot command
  for hook in client-attached after-new-window after-select-pane after-select-window; do
    while read -r slot command; do
      case "$command" in
        *"$PLUGIN_DIR"*) tmux set-hook -gu "$slot" ;;
      esac
    done < <(tmux show-hooks -g "$hook" 2>/dev/null || true)
  done
}

cleanup_sidebars() {
  local session sidebar
  while IFS= read -r session; do
    [ -n "$session" ] || continue
    sidebar="$(tmux show-option -qv -t "$session" @agent_watch_sidebar_pane 2>/dev/null || true)"
    if [ -n "$sidebar" ] &&
      [ "$(tmux show-option -pqv -t "$sidebar" @agent_watch_sidebar 2>/dev/null || true)" = 1 ]; then
      tmux kill-pane -t "$sidebar" 2>/dev/null || true
    fi
    tmux set-option -q -t "$session" @agent_watch_sidebar_pane ''
    tmux set-option -q -t "$session" @agent_watch_sidebar_expanded off
  done < <(tmux list-sessions -F '#{session_name}' 2>/dev/null || true)
}

cleanup_plugin_hooks
install_window_formats
if [ "$(option @agent-watch-sidebar off)" != on ]; then
  cleanup_sidebars
fi
if [ "$(option @agent-watch-hud on)" = on ]; then
  "$PLUGIN_DIR/scripts/hud-install.sh"
fi

tmux bind-key "$(option @agent-watch-next-key a)" run-shell "$PLUGIN_DIR/scripts/next-attention.sh"
tmux bind-key "$(option @agent-watch-sidebar-key Space)" run-shell "$PLUGIN_DIR/scripts/sidebar-resize.sh"
tmux bind-key "$(option @agent-watch-restart-key A)" run-shell "$PLUGIN_DIR/scripts/sidebar-restart.sh"
tmux bind-key "$(option @agent-watch-worktree-key W)" command-prompt -p 'Branch:' \
  "run-shell '$PLUGIN_DIR/scripts/worktree-new.sh --repo \"#{pane_current_path}\" \"%%\"'"
tmux bind-key "$(option @agent-watch-finish-key X)" display-popup -EE -w 70% -h 30% \
  -d '#{pane_current_path}' "$PLUGIN_DIR/scripts/worktree-finish.sh"
tmux bind-key "$(option @agent-watch-help-key H)" display-popup -E -w 72 -h 24 \
  "$PLUGIN_DIR/scripts/help.sh"
if [ "$(option @agent-watch-v2 on)" = on ]; then
  tmux bind-key "$(option @agent-watch-cockpit-key P)" display-popup -EE -w 96 -h 20 \
    -d '#{pane_current_path}' "$PLUGIN_DIR/scripts/v2.sh cockpit"
else
  tmux bind-key "$(option @agent-watch-cockpit-key P)" display-popup -EE -w 78 -h 26 \
    -d '#{pane_current_path}' "$PLUGIN_DIR/scripts/cockpit.sh"
fi
tmux bind-key "$(option @agent-watch-navigator-key w)" display-popup -EE -w 84 -h 24 \
  -d '#{pane_current_path}' "$PLUGIN_DIR/scripts/v2.sh navigator"
tmux bind-key "$(option @agent-watch-native-navigator-key C-w)" choose-tree -Zw
tmux bind-key "$(option @agent-watch-session-key s)" display-popup -EE -w 72 -h 18 \
  -d '#{pane_current_path}' "$PLUGIN_DIR/scripts/v2.sh sessions"
tmux bind-key "$(option @agent-watch-native-session-key C-s)" choose-tree -Zs
tmux bind-key '{' run-shell "$PLUGIN_DIR/scripts/safe-swap.sh -U"
tmux bind-key '}' run-shell "$PLUGIN_DIR/scripts/safe-swap.sh -D"
tmux bind-key -n MouseDown1Pane if-shell -F '#{==:#{@agent_watch_sidebar},1}' \
  "run-shell '$PLUGIN_DIR/scripts/sidebar-click.sh #{pane_id} #{mouse_y}'" \
  'select-pane -t ='
tmux bind-key -n WheelUpPane if-shell -F '#{==:#{@agent_watch_sidebar},1}' \
  'run-shell ":"' \
  'if-shell -F "#{||:#{pane_in_mode},#{mouse_any_flag}}" "send-keys -M" "copy-mode -e"'
tmux set-hook -g 'client-attached[100]' "run-shell -b '$PLUGIN_DIR/scripts/start-watcher.sh'"
if [ "$(option @agent-watch-v2 on)" = on ]; then
  tmux set-hook -g 'after-new-window[100]' "run-shell -b '$PLUGIN_DIR/scripts/v2.sh scan'"
else
  tmux set-hook -g 'after-new-window[100]' "run-shell -b '$PLUGIN_DIR/scripts/scan.sh'"
fi
tmux set-hook -g 'after-select-pane[100]' \
  "if-shell -F '#{==:#{@agent_watch_sidebar},1}' 'select-pane -l'"
tmux set-hook -g 'after-select-window[100]' "run-shell -b '$PLUGIN_DIR/scripts/sidebar-ensure.sh #{pane_id}'"

"$PLUGIN_DIR/scripts/start-watcher.sh"
