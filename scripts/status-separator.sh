#!/usr/bin/env bash

set -u

width="${1:-0}"
case "$width" in
  ''|*[!0-9]*) exit 0 ;;
esac

color="$(tmux show-option -gqv @agent-watch-separator-color 2>/dev/null || true)"
color="${color:-#393552}"

printf '#[fg=%s]' "$color"
awk -v width="$width" 'BEGIN { for (column = 0; column < width; column++) printf "─" }'
printf '#[default]'
