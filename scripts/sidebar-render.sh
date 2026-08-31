#!/usr/bin/env bash

set -u

session="$1"

color() {
  case "$1" in
    working) printf '\033[38;2;156;207;216m' ;;
    needs_input) printf '\033[38;2;246;193;119m' ;;
    done) printf '\033[38;2;166;218;149m' ;;
    failed) printf '\033[38;2;235;111;146m' ;;
    *) printf '\033[38;2;144;140;170m' ;;
  esac
}

age() {
  local since="$1" elapsed
  [ -n "$since" ] || { printf '-'; return; }
  elapsed=$(($(date +%s) - since))
  if [ "$elapsed" -lt 60 ]; then printf '<1m'
  elif [ "$elapsed" -lt 3600 ]; then printf '%sm' "$((elapsed / 60))"
  else printf '%sh' "$((elapsed / 3600))"
  fi
}

agent_row() {
  local name="$1" state="$2" current="$3" width="$4"
  if [ "$current" = 1 ]; then printf '\033[48;2;57;53;82m'; fi
  printf ' %b●\033[0m%s %-*.*s' "$(color "$state")" \
    "$([ "$current" = 1 ] && printf '\033[48;2;57;53;82m' || true)" \
    "$width" "$width" "$name"
  printf '\033[0m\n'
}

compact_dot() {
  local state="$1" current="$2"
  if [ "$current" = 1 ]; then printf '\033[48;2;57;53;82m'; fi
  printf ' %b●\033[0m' "$(color "$state")"
  if [ "$current" = 1 ]; then printf '\033[48;2;57;53;82m'; fi
  printf ' \033[0m\n'
}

display_path() {
  local path="$1"
  case "$path" in
    "$HOME") printf '~' ;;
    "$HOME"/*) printf '~/%s' "${path#"$HOME"/}" ;;
    *) printf '%s' "$path" ;;
  esac
}

state_label() {
  case "$1" in
    working) printf 'WORKING' ;;
    needs_input) printf 'WAITING FOR YOU' ;;
    done) printf 'READY TO REVIEW' ;;
    failed) printf 'FAILED' ;;
  esac
}

useful_context() {
  local message="$1"
  case "$message" in
    *'Worked for'*|*'Context '*' used'*|*'gpt-'*|'') return ;;
    *) printf '%s' "$message" ;;
  esac
}

render_context() {
  local context="$1" window_id="$2"
  while IFS= read -r line; do
    printf '    \033[38;2;224;222;244m%s\033[0m\n' "$line"
    click_map="${click_map}${row_number}=${window_id};"
    row_number=$((row_number + 1))
  done < <(printf '%s\n' "$context" | fold -s -w 30)
}

previous_frame=''
while tmux has-session -t "$session" 2>/dev/null; do
  expanded="$(tmux show-option -qv -t "$session" @agent_watch_sidebar_expanded 2>/dev/null || true)"
  current_window="$(tmux display-message -p -t "$session:" '#{window_id}')"
  rows="$(tmux list-windows -t "$session" -F '#{window_id}|#{window_name}|#{@agent_watch_state}|#{@agent_watch_since}|#{@agent_watch_message}|#{pane_current_path}')"
  click_map=''
  row_number=0

  if [ "$expanded" != on ]; then
    frame="$(
      while IFS='|' read -r window_id name state since message path; do
        [ -n "$state" ] || continue
        compact_dot "$state" "$([ "$window_id" = "$current_window" ] && printf 1 || printf 0)"
        click_map="${click_map}${row_number}=${window_id};"
        row_number=$((row_number + 1))
      done <<< "$rows"
      printf '\034%s' "$click_map"
    )"
  else
    frame="$(
      printf '\033[1m AGENTS\033[0m  \033[2m%s\033[0m\n\n' "$session"
      row_number=2
      attention="$(printf '%s\n' "$rows" | awk -F '|' '$3=="needs_input" || $3=="done" || $3=="failed"')"
      if [ -n "$attention" ]; then
        printf ' \033[1mNEEDS YOU\033[0m\n'
        row_number=$((row_number + 1))
        while IFS='|' read -r window_id name state since message path; do
          agent_row "$name" "$state" "$([ "$window_id" = "$current_window" ] && printf 1 || printf 0)" 27
          click_map="${click_map}${row_number}=${window_id};$((row_number + 1))=${window_id};$((row_number + 2))=${window_id};"
          row_number=$((row_number + 1))
          printf '    \033[38;2;156;207;216m%.30s\033[0m\n' "$(display_path "$path")"
          row_number=$((row_number + 1))
          printf '    %b%s\033[0m · %s\n' "$(color "$state")" "$(state_label "$state")" "$(age "$since")"
          row_number=$((row_number + 1))
          context="$(useful_context "$message")"
          [ -n "$context" ] && render_context "$context" "$window_id"
          printf '\n'; row_number=$((row_number + 1))
        done <<< "$attention"
      fi

      working="$(printf '%s\n' "$rows" | awk -F '|' '$3=="working"')"
      if [ -n "$working" ]; then
        printf ' \033[1mWORKING\033[0m\n'
        row_number=$((row_number + 1))
        while IFS='|' read -r window_id name state since message path; do
          agent_row "$name" "$state" "$([ "$window_id" = "$current_window" ] && printf 1 || printf 0)" 27
          click_map="${click_map}${row_number}=${window_id};$((row_number + 1))=${window_id};$((row_number + 2))=${window_id};"
          row_number=$((row_number + 1))
          printf '    \033[38;2;156;207;216m%.30s\033[0m\n' "$(display_path "$path")"
          row_number=$((row_number + 1))
          printf '    %b%s\033[0m · %s\n' "$(color "$state")" "$(state_label "$state")" "$(age "$since")"
          row_number=$((row_number + 1))
          context="$(useful_context "$message")"
          [ -n "$context" ] && render_context "$context" "$window_id"
        done <<< "$working"
      fi
      printf '\034%s' "$click_map"
    )"
  fi

  click_map="${frame##*$'\034'}"
  frame="${frame%$'\034'*}"
  tmux set-option -pq -t "$TMUX_PANE" @agent_watch_click_map "$click_map"
  if [ "$frame" != "$previous_frame" ]; then
    printf '\033[H\033[J%s' "$frame"
    previous_frame="$frame"
  fi
  sleep 1
done
