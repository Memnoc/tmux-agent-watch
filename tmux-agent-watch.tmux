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
  for hook in client-attached after-new-window after-select-window; do
    while read -r slot command; do
      case "$command" in
        *"$PLUGIN_DIR"*) tmux set-hook -gu "$slot" ;;
      esac
    done < <(tmux show-hooks -g "$hook" 2>/dev/null || true)
  done
}

cleanup_plugin_hooks
install_window_formats
if [ "$(option @agent-watch-hud on)" = on ]; then
  "$PLUGIN_DIR/scripts/hud-install.sh"
fi

tmux bind-key "$(option @agent-watch-next-key a)" run-shell "$PLUGIN_DIR/scripts/next-attention.sh"
tmux bind-key "$(option @agent-watch-sidebar-key Space)" run-shell "$PLUGIN_DIR/scripts/sidebar-resize.sh"
tmux bind-key -n MouseDown1Pane if-shell -F '#{==:#{@agent_watch_sidebar},1}' \
  "run-shell '$PLUGIN_DIR/scripts/sidebar-click.sh #{pane_id} #{mouse_y}'" \
  'select-pane -t ='
tmux set-hook -g 'client-attached[100]' "run-shell -b '$PLUGIN_DIR/scripts/start-watcher.sh'"
tmux set-hook -g 'after-new-window[100]' "run-shell -b '$PLUGIN_DIR/scripts/scan.sh'"
tmux set-hook -g 'after-select-window[100]' "run-shell -b '$PLUGIN_DIR/scripts/sidebar-ensure.sh #{pane_id}'"

"$PLUGIN_DIR/scripts/start-watcher.sh"
