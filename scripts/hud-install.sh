#!/usr/bin/env bash

set -eu

PLUGIN_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"

bar="#($PLUGIN_DIR/scripts/status-bar.sh '#{session_name}' '#{window_id}' '#{client_width}')"
separator="#($PLUGIN_DIR/scripts/status-separator.sh '#{client_width}')"

tmux set-option -g status 2
tmux set-option -g status-style 'bg=default,fg=default'
if [ "$(tmux show-option -gqv status-position)" = bottom ]; then
  tmux set-option -g status-format[0] "$separator"
  tmux set-option -g status-format[1] "$bar"
else
  tmux set-option -g status-format[0] "$bar"
  tmux set-option -g status-format[1] "$separator"
fi
