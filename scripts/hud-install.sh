#!/usr/bin/env bash

set -eu

PLUGIN_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"

saved="$(tmux show-option -gqv @drudwyn_hud_saved)"
if [ "${1:-}" = --disable ]; then
  if [ "$saved" = on ]; then
    for name in status status-style; do
      tmux set-option -g "$name" "$(tmux show-option -gqv "@drudwyn_saved_$name")"
      tmux set-option -gu "@drudwyn_saved_$name"
    done
    tmux set-option -gu status-format
    for index in $(tmux show-option -gqv @drudwyn_saved_status_indexes); do
      tmux set-option -g "status-format[$index]" "$(tmux show-option -gqv "@drudwyn_saved_status_format_$index")"
      tmux set-option -gu "@drudwyn_saved_status_format_$index"
    done
    tmux set-option -gu @drudwyn_saved_status_indexes
    tmux set-option -gu @drudwyn_hud_saved
  elif tmux show-option -g status-format | grep -Eq "$PLUGIN_DIR/scripts/status-(bar|a).sh"; then
    # Older installs did not save the replaced status configuration.
    for name in status status-style status-format; do tmux set-option -gu "$name"; done
  fi
  exit 0
fi

if [ "$saved" != on ] && ! tmux show-option -g status-format | grep -Eq "$PLUGIN_DIR/scripts/status-(bar|a).sh"; then
  for name in status status-style; do
    tmux set-option -gq "@drudwyn_saved_$name" "$(tmux show-option -gqv "$name")"
  done
  indexes=''
  while read -r name rest; do
    index="${name#status-format[}"; index="${index%]}"
    indexes="$indexes $index"
    tmux set-option -gq "@drudwyn_saved_status_format_$index" "$(tmux show-option -gqv "$name")"
  done < <(tmux show-option -g status-format)
  tmux set-option -gq @drudwyn_saved_status_indexes "$indexes"
  tmux set-option -gq @drudwyn_hud_saved on
fi

bar="#($PLUGIN_DIR/scripts/status-bar.sh '#{session_name}' '#{window_id}' '#{client_width}')"
separator="#($PLUGIN_DIR/scripts/status-separator.sh '#{client_width}')"

tmux set-option -g status 2
tmux set-option -g status-style 'bg=default,fg=default'
if [ "$(tmux show-option -gqv @drudwyn-v2)" != off ]; then
  if [ "$(tmux show-option -gqv @drudwyn-status-layout)" != tabs ]; then
    tmux set-option -g status on
    tmux set-option -g 'status-format[0]' "#($PLUGIN_DIR/scripts/status-a.sh '#{session_id}' '#{window_id}' '#{client_width}' focus)"
    tmux set-option -gu 'status-format[1]'
    exit 0
  fi
  # Both jobs are keyed by this client's stable selection and terminal width.
  # Both rows reuse the observer projection; redraws never start process scans.
  tmux set-option -g 'status-format[0]' "#($PLUGIN_DIR/scripts/status-a.sh '#{session_id}' '#{window_id}' '#{client_width}' tabs)"
  tmux set-option -g 'status-format[1]' "#($PLUGIN_DIR/scripts/status-a.sh '#{session_id}' '#{window_id}' '#{client_width}' context)"
  exit 0
fi
if [ "$(tmux show-option -gqv status-position)" = bottom ]; then
  tmux set-option -g status-format[0] "$separator"
  tmux set-option -g status-format[1] "$bar"
else
  tmux set-option -g status-format[0] "$bar"
  tmux set-option -g status-format[1] "$separator"
fi
