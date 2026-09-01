#!/usr/bin/env bash

set -eu
ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"

bash -n "$ROOT/tmux-agent-watch.tmux" "$ROOT"/scripts/*.sh "$ROOT"/tests/*.sh
"$ROOT/tests/classify_test.sh"
"$ROOT/tests/help_test.sh"
"$ROOT/tests/integration_test.sh"
"$ROOT/tests/worktree_test.sh"
