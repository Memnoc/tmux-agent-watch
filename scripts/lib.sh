#!/usr/bin/env bash

PLUGIN_DIR="$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"

tmux_option() {
  local name="$1" default="$2" value
  value="$(tmux show-option -gqv "$name" 2>/dev/null || true)"
  printf '%s' "${value:-$default}"
}

window_option() {
  tmux show-option -wqv -t "$1" "$2" 2>/dev/null || true
}

is_agent_command() {
  case "${1##*/}" in
    codex|claude|opencode) return 0 ;;
    *) return 1 ;;
  esac
}

strip_terminal_noise() {
  LC_ALL=C sed \
    -e $'s/\033\[[0-9;?]*[ -\/]*[@-~]//g' \
    -e $'s/\r//g' \
    -e 's/[[:space:]]\+$//' \
    -e '/^[[:space:]]*$/d'
}

classify_output() {
  local command="$1" output="$2" tail_text last_line

  if ! is_agent_command "$command"; then
    printf 'unmanaged\t'
    return
  fi

  tail_text="$(printf '%s\n' "$output" | strip_terminal_noise | tail -n 14)"
  last_line="$(printf '%s\n' "$tail_text" | tail -n 1)"

  if printf '%s\n' "$tail_text" | grep -Eqi \
    '(esc to interrupt|ctrl.c to interrupt|working\.\.\.|thinking\.\.\.|baking…|running command|tokens[^[:alnum:]]*$)'; then
    printf 'working\t%s' "$last_line"
  elif printf '%s\n' "$tail_text" | grep -Eqi \
    '(do you want to proceed|allow (this )?(command|action)|approve|permission|waiting for (your )?(input|approval)|please (choose|confirm)|[?][[:space:]]*$)'; then
    printf 'needs_input\t%s' "$last_line"
  elif printf '%s\n' "$tail_text" | grep -Eqi \
    '(^|[[:space:]])(error|failed|fatal|panic|traceback)(:|[[:space:]])|[1-9][0-9]* failed'; then
    printf 'failed\t%s' "$last_line"
  elif printf '%s\n' "$tail_text" | tail -n 6 | grep -Eq '^[[:space:]›❯>]+$|^[[:space:]]*[›❯][[:space:]]*$'; then
    printf 'done\t%s' "$(printf '%s\n' "$tail_text" | tail -n 7 | grep -Ev '^[[:space:]›❯>]+$|^[[:space:]]*[›❯][[:space:]]*$' | tail -n 1)"
  else
    printf 'working\t%s' "$last_line"
  fi
}

symbol_for_state() {
  case "$1" in
    working) tmux_option @agent-watch-working-symbol '●' ;;
    needs_input) tmux_option @agent-watch-needs-input-symbol '?' ;;
    done) tmux_option @agent-watch-done-symbol '✓' ;;
    failed) tmux_option @agent-watch-failed-symbol '!' ;;
  esac
}

color_for_state() {
  case "$1" in
    working) tmux_option @agent-watch-working-color '#9ccfd8' ;;
    needs_input) tmux_option @agent-watch-needs-input-color '#f6c177' ;;
    done) tmux_option @agent-watch-done-color '#a6da95' ;;
    failed) tmux_option @agent-watch-failed-color '#ed8796' ;;
  esac
}

set_window_state() {
  local window_id="$1" state="$2" message="${3:-}" previous now marker color symbol
  previous="$(window_option "$window_id" @agent_watch_state)"
  now="$(date +%s)"

  if [ "$state" = unmanaged ]; then
    tmux set-option -wq -t "$window_id" @agent_watch_state ''
    tmux set-option -wq -t "$window_id" @agent_watch_marker ''
    tmux set-option -wq -t "$window_id" @agent_watch_message ''
    return
  fi

  if [ "$state" != "$previous" ]; then
    tmux set-option -wq -t "$window_id" @agent_watch_since "$now"
    case "$state" in
      needs_input|done|failed)
        tmux set-option -wq -t "$window_id" @agent_watch_attention_since "$now"
        if [ -n "$previous" ]; then
          tmux display-message -d 3000 "Agent $(tmux display-message -p -t "$window_id" '#W'): ${state//_/ }"
        fi
        ;;
      *) tmux set-option -wq -t "$window_id" @agent_watch_attention_since '' ;;
    esac
  fi

  symbol="$(symbol_for_state "$state")"
  color="$(color_for_state "$state")"
  marker="#[fg=${color}]${symbol}#[default] "
  tmux set-option -wq -t "$window_id" @agent_watch_state "$state"
  tmux set-option -wq -t "$window_id" @agent_watch_marker "$marker"
  message="${message//|/¦}"
  tmux set-option -wq -t "$window_id" @agent_watch_message "${message:0:120}"
}
