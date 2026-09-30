#!/usr/bin/env bash
# Inject only the command failure; all window/process behavior remains real tmux.
if [ "${1:-}" = set-option ]; then
  printf 'injected metadata attachment failure\n' >&2
  exit 1
fi
exec "${DRUDWYN_TEST_TMUX:?}" "$@"
