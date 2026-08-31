#!/usr/bin/env bash

set -eu
source "$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)/scripts/lib.sh"

failures=0

assert_state() {
  local expected="$1" command="$2" fixture="$3" actual
  actual="$(classify_output "$command" "$fixture")"
  actual="${actual%%$'\t'*}"
  if [ "$actual" != "$expected" ]; then
    printf 'not ok: expected %s, got %s\n' "$expected" "$actual"
    failures=$((failures + 1))
  else
    printf 'ok: %s\n' "$expected"
  fi
}

assert_state unmanaged zsh 'ordinary shell output'
assert_state working codex $'• Exploring repository\n  esc to interrupt'
assert_state needs_input claude $'Bash command\nDo you want to proceed?\n❯'
assert_state done codex $'• Tests pass. Ready for review.\n›'
assert_state done opencode $'Test suite failed: 2 failed\n›'

# Regressions captured from live Codex panes. Recovered command errors are not
# agent failures, the text after › is user input, and Codex renders an idle
# placeholder instead of a bare prompt.
assert_state done codex $'error connecting to /tmp/tmux-1000/default\n• Plugin is live and ready.\n› Ask Codex to do anything\n~ · gpt-5.6-sol default · Context 14% used'
assert_state done codex $'• Plugin is live and ready.\n› is this working?\n~ · gpt-5.6-sol default · Context 14% used'
assert_state needs_input codex $'• Shall I apply the migration?\n› Ask Codex to do anything\n~ · gpt-5.6-sol default · Context 14% used'

if [ "$failures" -ne 0 ]; then
  exit 1
fi
