#!/usr/bin/env bash

set -eu
ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"

bash -n "$ROOT/tmux-drudwyn.tmux" "$ROOT"/scripts/*.sh "$ROOT"/tests/*.sh
bash "$ROOT/tests/rename_test.sh"
bash "$ROOT/tests/icons_test.sh"
"$ROOT/tests/classify_test.sh"
"$ROOT/tests/help_test.sh"
"$ROOT/tests/status_bar_test.sh"
"$ROOT/tests/integration_test.sh"
"$ROOT/tests/worktree_test.sh"
"$ROOT/tests/cockpit_test.sh"
"$ROOT/tests/v2_test.sh"
bash "$ROOT/tests/start_failure_test.sh"
bash "$ROOT/tests/lifecycle_test.sh"
python3 "$ROOT/tests/navigator_test.py"
python3 "$ROOT/tests/independent_navigation_test.py"
python3 "$ROOT/tests/settings_test.py"
"$ROOT/tests/privacy_test.sh"
"$ROOT/tests/package_test.sh"
"$ROOT/tests/release_workflow_test.sh"
