#!/usr/bin/env bash

set -u
source "$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)/lib.sh"

tmp_file="$(mktemp)"
choices_file="${tmp_file}.choices"
trap 'rm -f "$tmp_file" "$choices_file"' EXIT

tmux list-windows -a -F '#{session_name}|#{window_id}|#{window_name}|#{@agent_watch_state}|#{@agent_watch_since}|#{@agent_watch_message}' |
  awk -F '|' '$4 != ""' > "$tmp_file"

if [ ! -s "$tmp_file" ]; then
  printf '\n  No active agents detected.\n\n  Press Enter to close. '
  read -r _
  exit 0
fi

printf '\n  AGENTS\n\n'
number=0
while IFS='|' read -r session window_id name state since message; do
  number=$((number + 1))
  symbol="$(symbol_for_state "$state")"
  printf '  %2d  %s  %-18s %-12s %s\n' "$number" "$symbol" "$session:$name" "${state//_/ }" "$message"
  printf '%s|%s\n' "$number" "$session:$window_id" >> "$choices_file"
done < <(awk -F '|' 'BEGIN { OFS="|" } { rank=($4=="needs_input"?1:$4=="failed"?2:$4=="done"?3:4); print rank,$0 }' "$tmp_file" | sort -t '|' -k1,1n -k6,6n | cut -d '|' -f2-)

printf '\n  Select a window (Enter closes): '
read -r choice
[ -n "$choice" ] || exit 0
target="$(awk -F '|' -v choice="$choice" '$1 == choice { print $2; exit }' "$choices_file")"
[ -n "$target" ] || exit 0
tmux switch-client -t "$target"
