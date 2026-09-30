#!/usr/bin/env bash

set -eu

ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
TMP_DIR="$(mktemp -d)"

cleanup() {
  tmux -L "drudwyn-help-default-$$" kill-server 2>/dev/null || true
  tmux -L "drudwyn-help-custom-$$" kill-server 2>/dev/null || true
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

start_tmux() {
  local socket="$1"
  tmux -L "$socket" -f /dev/null new-session -d -s help
  tmux -L "$socket" set-option -g @drudwyn-hud off
  tmux -L "$socket" set-option -g @drudwyn-sidebar off
}

default_socket="drudwyn-help-default-$$"
start_tmux "$default_socket"
tmux -L "$default_socket" run-shell "$ROOT/tmux-drudwyn.tmux"
default_binding="$(tmux -L "$default_socket" list-keys -T prefix | awk '$4 == "H" && /scripts\/help.sh/')"
[ -n "$default_binding" ] || {
  printf 'not ok: default prefix + H help binding missing\n'
  exit 1
}
printf 'ok: help is bound to prefix + H by default\n'

custom_socket="drudwyn-help-custom-$$"
start_tmux "$custom_socket"
tmux -L "$custom_socket" set-option -g @drudwyn-help-key '?'
tmux -L "$custom_socket" run-shell "$ROOT/tmux-drudwyn.tmux"
custom_binding="$(tmux -L "$custom_socket" list-keys -T prefix | awk '$4 == "?" && /scripts\/help.sh/')"
[ -n "$custom_binding" ] || {
  printf 'not ok: configured help binding missing\n'
  exit 1
}
printf 'ok: @drudwyn-help-key configures the help binding\n'

help_output="$(printf q | "$ROOT/scripts/help.sh")"
for expected in \
  '●  normal agent' \
  '◆  agent in a linked Git worktree' \
  'a      jump' \
  'P      open the workspace cockpit' \
  'W      create' \
  'X      finish' \
  'm      zoom' \
  'w      open the grouped workspace navigator' \
  's      open the compact session navigator' \
  'O      customise Drudwyn options' \
  's      save all sessions with tmux-resurrect' \
  'n      new shell session in the session navigator' \
  'C-s    save all sessions (tmux-resurrect)' \
  'C-w    open the native tmux window tree' \
  'left   ordinary workspaces' \
  'centre current Git changes' \
  'right  managed agents' \
  'CLEAN  no uncommitted changes' \
  'DIRTY  has uncommitted changes' \
  '[q/Esc] Close'
do
  printf '%s\n' "$help_output" | grep -Fq "$expected" || {
    printf 'not ok: help output missing %s\n' "$expected"
    exit 1
  }
done
printf 'ok: help renders controls, markers, and worktree states\n'

printf '\033' | "$ROOT/scripts/help.sh" >/dev/null
printf 'ok: help closes with q or Escape\n'
