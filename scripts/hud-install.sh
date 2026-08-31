#!/usr/bin/env bash

set -eu

PLUGIN_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"

tmux set-option -g status on
tmux set-option -g status-style 'bg=default,fg=default'
tmux set-option -g status-format[0] "#[align=left]#($PLUGIN_DIR/scripts/hud.sh fleet '#{session_name}' '#{window_id}')#[align=right]#($PLUGIN_DIR/scripts/hud.sh selected '#{session_name}' '#{window_id}') "
