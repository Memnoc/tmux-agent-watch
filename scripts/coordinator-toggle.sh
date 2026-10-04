#!/usr/bin/env bash
set -eu
PLUGIN_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
export DRUDWYN_CLIENT="${1:?requesting client required}"
if ! result="$("$PLUGIN_DIR/scripts/v2.sh" coordinator toggle 2>&1)"; then
  tmux display-message -c "$DRUDWYN_CLIENT" -d 6000 "$result"
fi
