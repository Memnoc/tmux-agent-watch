#!/usr/bin/env bash

set -u

mode="$1"
session="${2:-}"
window_id="${3:-}"

state_color() {
  case "$1" in
    working) printf '#9ccfd8' ;;
    needs_input) printf '#f6c177' ;;
    done) printf '#a6da95' ;;
    failed) printf '#eb6f92' ;;
    *) printf '#908caa' ;;
  esac
}

state_label() {
  case "$1" in
    working) printf 'WORKING' ;;
    needs_input) printf 'WAITING FOR YOU' ;;
    done) printf 'READY TO REVIEW' ;;
    failed) printf 'FAILED' ;;
    *) printf 'SHELL' ;;
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

case "$mode" in
  fleet)
    rows="$(tmux list-windows -t "$session" -F '#{@agent_watch_state}|#{@agent_watch_attention_since}' 2>/dev/null || true)"
    agents="$(printf '%s\n' "$rows" | awk -F '|' '$1!="" {n++} END {print n+0}')"
    working="$(printf '%s\n' "$rows" | awk -F '|' '$1=="working" {n++} END {print n+0}')"
    waiting="$(printf '%s\n' "$rows" | awk -F '|' '$1=="needs_input" {n++} END {print n+0}')"
    review="$(printf '%s\n' "$rows" | awk -F '|' '$1=="done" {n++} END {print n+0}')"
    failed="$(printf '%s\n' "$rows" | awk -F '|' '$1=="failed" {n++} END {print n+0}')"
    oldest="$(printf '%s\n' "$rows" | awk -F '|' '$2!="" && (oldest=="" || $2<oldest) {oldest=$2} END {print oldest}')"

    printf '#[fg=#c4a7e7,bold] %s #[default]  %s agents' "$session" "$agents"
    [ "$working" -gt 0 ] && printf '  #[fg=#9ccfd8]● %s working#[default]' "$working"
    [ "$waiting" -gt 0 ] && printf '  #[fg=#f6c177]● %s waiting#[default]' "$waiting"
    [ "$review" -gt 0 ] && printf '  #[fg=#a6da95]● %s review#[default]' "$review"
    [ "$failed" -gt 0 ] && printf '  #[fg=#eb6f92]● %s failed#[default]' "$failed"
    [ -n "$oldest" ] && printf '  #[fg=#908caa]oldest %s#[default]' "$(age "$oldest")"
    ;;
  selected)
    details="$(tmux display-message -p -t "$window_id" '#{window_name}|#{pane_current_path}|#{pane_current_command}|#{@agent_watch_state}|#{@agent_watch_since}' 2>/dev/null || true)"
    IFS='|' read -r name path command state since <<< "$details"
    branch="$(git -C "$path" branch --show-current 2>/dev/null || true)"
    dirty=''
    if [ -n "$branch" ] && [ -n "$(git -C "$path" status --porcelain 2>/dev/null || true)" ]; then dirty='*'; fi
    color="$(state_color "$state")"
    printf '#[fg=#e0def4,bold] %s#[default]' "${name:-shell}"
    [ -n "$branch" ] && printf '  #[fg=#9ccfd8]%s%s#[default]' "$branch" "$dirty"
    printf '  #[fg=#908caa]%s#[default]' "${command:-shell}"
    printf '  #[fg=%s,bold]%s#[default] #[fg=#908caa]· %s#[default]' "$color" "$(state_label "$state")" "$(age "$since")"
    ;;
  summary)
    message="$(tmux show-option -wqv -t "$window_id" @agent_watch_message 2>/dev/null || true)"
    if [ -n "$message" ]; then
      printf '#[fg=#908caa] ↳ #[fg=#e0def4]%s#[default]' "$message"
    else
      path="$(tmux display-message -p -t "$window_id" '#{pane_current_path}' 2>/dev/null || true)"
      printf '#[fg=#908caa] ↳ %s#[default]' "$path"
    fi
    ;;
esac

exit 0
