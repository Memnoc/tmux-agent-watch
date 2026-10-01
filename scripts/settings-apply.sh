#!/usr/bin/env bash

# Apply the runtime effects of a validated option. Called directly, rather than
# through tmux's command queue, so this also works inside a blocking popup.
set -eu
PLUGIN_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
option="$1"

if [ "$option" = @drudwyn-interval ]; then
  bash "$PLUGIN_DIR/scripts/start-watcher.sh" --restart
fi

case "$option" in
  @drudwyn-*-key|@drudwyn-hud|@drudwyn-status|@drudwyn-status-layout|@drudwyn-color-window-names|@drudwyn-sidebar|@drudwyn-v2)
    bash "$PLUGIN_DIR/tmux-drudwyn.tmux"
    ;;
esac

case "$option" in
  @drudwyn-sidebar-width|@drudwyn-sidebar-expanded-width|@drudwyn-v2)
    while IFS='|' read -r pane session expanded; do
      [ -n "$pane" ] || continue
      if [ "$option" = @drudwyn-v2 ]; then
        renderer=sidebar-v2.sh
        [ "$(tmux show-option -gqv @drudwyn-v2)" != off ] || renderer=sidebar-render.sh
        session_name="$(tmux display-message -p -t "$pane" '#{session_name}')"
        tmux respawn-pane -k -t "$pane" "$PLUGIN_DIR/scripts/$renderer" "$session_name"
      else
        width_option=@drudwyn-sidebar-width; fallback=3
        if [ "$expanded" = on ]; then width_option=@drudwyn-sidebar-expanded-width; fallback=38; fi
        width="$(tmux show-option -gqv "$width_option")"
        tmux resize-pane -t "$pane" -x "${width:-$fallback}"
      fi
    done < <(tmux list-panes -a -F '#{?@drudwyn_sidebar,#{pane_id}|#{session_id}|#{@drudwyn_sidebar_expanded},}' | sed '/^$/d')
    ;;
esac

case "$option" in
  @drudwyn-sidebar|@drudwyn-sidebar-width|@drudwyn-sidebar-expanded-width|@drudwyn-v2)
    while IFS= read -r pane; do
      bash "$PLUGIN_DIR/scripts/sidebar-ensure.sh" "$pane"
    done < <(tmux list-panes -a -F '#{?window_active,#{?pane_active,#{pane_id},},}' | sed '/^$/d')
    ;;
esac

while IFS= read -r client; do
  tmux refresh-client -S -t "$client"
done < <(tmux list-clients -F '#{client_name}')
