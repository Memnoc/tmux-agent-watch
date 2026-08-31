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

clean_summary_line() {
  sed -E \
    -e 's/^[[:space:]]*[•●›-][[:space:]]*//' \
    -e 's/[[:space:]]+/ /g' \
    -e 's/^[[:space:]]+//' \
    -e 's/[[:space:]]+$//'
}

extract_user_task() {
  awk '
    /^[[:space:]]*›[[:space:]]+/ {
      line=$0
      sub(/^[[:space:]]*›[[:space:]]+/, "", line)
      collecting=0
      if (line !~ /^Ask (Codex|Claude)/ && line != "") {
        task=line
        collecting=1
      }
      next
    }
    collecting {
      if ($0 ~ /^[[:space:]]+(•|●|✻|~[[:space:]]*·|esc to interrupt|ctrl.c to interrupt)/) {
        collecting=0
        next
      }
      if ($0 ~ /^[[:space:]]+/) {
        line=$0
        sub(/^[[:space:]]+/, "", line)
        task=task " " line
      } else {
        collecting=0
      }
    }
    END { print task }
  ' | clean_summary_line
}

extract_completed_summary() {
  sed -E \
    -e 's/^[[:space:]]*[•●][[:space:]]+/__AGENT__/' \
    -e 's/^─+/__DIVIDER__/' |
  awk '
    /(Worked|Brewed) for/ {
      if (segment != "") completed=segment
      segment=""
      next
    }
    /^__DIVIDER__/ { segment=""; next }
    /^__AGENT__/ && segment == "" {
      line=$0
      sub(/^__AGENT__/, "", line)
      if (line !~ /^(Ran|Running|Explored|Edited|Waiting|Completed `\/root|Updated Plan|Started)/) segment=line
    }
    END { print completed }
  ' | clean_summary_line
}

classify_output() {
  local command="$1" output="$2" cleaned tail_text last_line response prompt_seen summary

  if ! is_agent_command "$command"; then
    printf 'unmanaged\t'
    return
  fi

  cleaned="$(printf '%s\n' "$output" | strip_terminal_noise)"
  tail_text="$(printf '%s\n' "$cleaned" | tail -n 14)"
  last_line="$(printf '%s\n' "$tail_text" | tail -n 1)"

  if printf '%s\n' "$tail_text" | grep -Eqi \
    '(esc to interrupt|ctrl.c to interrupt|working\.\.\.|thinking\.\.\.|baking…|running command)'; then
    summary="$(printf '%s\n' "$cleaned" | extract_user_task)"
    printf 'working\t%s' "${summary:-Working}"
  elif printf '%s\n' "$tail_text" | grep -Eqi \
    '(do you want to proceed|allow (this )?(command|action)|approve|permission|waiting for (your )?(input|approval)|please (choose|confirm)|let me know what|what would you like|would you like|want me to)'; then
    summary="$(printf '%s\n' "$tail_text" | grep -Ei '(do you want to proceed|allow (this )?(command|action)|approve|permission|waiting for (your )?(input|approval)|please (choose|confirm)|let me know what|what would you like|would you like|want me to)' | tail -n 1 | clean_summary_line)"
    printf 'needs_input\t%s' "${summary:-Needs your input}"
  else
    # Codex and Claude keep their input prompt visible at the bottom. Text on
    # that line is a placeholder or user input, never an agent question. Find
    # the final prompt and inspect only the response immediately before it.
    prompt_seen="$(printf '%s\n' "$tail_text" | grep -Ec '^[[:space:]]*[›❯>]')"
    if [ "$prompt_seen" -gt 0 ]; then
      response="$(printf '%s\n' "$tail_text" | awk '
        { line[NR]=$0; if ($0 ~ /^[[:space:]]*[›❯>]/) prompt=NR }
        END {
          start=(prompt > 8 ? prompt - 8 : 1)
          for (i=start; i<prompt; i++) print line[i]
        }')"

      if printf '%s\n' "$response" | tail -n 4 | grep -Eq '[?][[:space:]]*$'; then
        summary="$(printf '%s\n' "$response" | tail -n 1 | clean_summary_line)"
        printf 'needs_input\t%s' "${summary:-Needs your input}"
      else
        summary="$(printf '%s\n' "$cleaned" | extract_completed_summary)"
        if [ -z "$summary" ]; then
          summary="$(printf '%s\n' "$response" | grep -Ev '^[[:space:]]*$' | head -n 1 | clean_summary_line)"
        fi
        printf 'done\t%s' "${summary:-Ready for review}"
      fi
    else
      printf 'working\t%s' "$last_line"
    fi
  fi
}

symbol_for_state() {
  case "$1" in
    working) tmux_option @agent-watch-working-symbol '●' ;;
    needs_input) tmux_option @agent-watch-needs-input-symbol '●' ;;
    done) tmux_option @agent-watch-done-symbol '●' ;;
    failed) tmux_option @agent-watch-failed-symbol '●' ;;
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
  if [ "$state" = working ] && [ "$previous" = working ] && [ "$message" = Working ]; then
    message="$(window_option "$window_id" @agent_watch_message)"
    message="${message:-Working}"
  fi
  now="$(date +%s)"

  if [ "$state" = unmanaged ]; then
    tmux set-option -wq -t "$window_id" @agent_watch_state ''
    tmux set-option -wq -t "$window_id" @agent_watch_marker ''
    tmux set-option -wq -t "$window_id" @agent_watch_window_style ''
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
  tmux set-option -wq -t "$window_id" @agent_watch_window_style "#[fg=${color}]"
  message="${message//|/¦}"
  tmux set-option -wq -t "$window_id" @agent_watch_message "$message"
}
