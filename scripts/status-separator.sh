#!/usr/bin/env bash

set -u

width="${1:-0}"
case "$width" in
  ''|*[!0-9]*) exit 0 ;;
esac

color="$(tmux show-option -gqv @drudwyn-separator-color 2>/dev/null || true)"
case "$color" in
  ''|default)
    case "$(tmux show-option -gqv @drudwyn-theme 2>/dev/null || true)" in
      dawn) color='#dfdad9' ;;
      moon) color='#393552' ;;
      *) color='#403d52' ;;
    esac
    ;;
esac

printf '#[fg=%s]' "$color"
awk -v width="$width" 'BEGIN { for (column = 0; column < width; column++) printf "─" }'
printf '#[default]'
