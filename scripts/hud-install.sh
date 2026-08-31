#!/usr/bin/env bash

set -eu

PLUGIN_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"

tmux set-option -g status 3
tmux set-option -g status-style 'bg=default,fg=default'
tmux set-option -g status-format[0] "#[align=left]#($PLUGIN_DIR/scripts/hud.sh fleet '#{session_name}' '#{window_id}')#[align=right]#[fg=#908caa]%H:%M "
tmux set-option -g status-format[1] "#[align=left]#($PLUGIN_DIR/scripts/hud.sh selected '#{session_name}' '#{window_id}')"
tmux set-option -g status-format[2] "#[align=left]#($PLUGIN_DIR/scripts/hud.sh summary '#{session_name}' '#{window_id}')"
