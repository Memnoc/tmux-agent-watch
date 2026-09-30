#!/usr/bin/env bash
set -eu
PLUGIN_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
if [ "$(tmux show-option -gqv @drudwyn-v2)" = off ]; then
  exec tmux select-window -t "${1:?window ID required}"
fi
# The rendered range holds the window ID, so index reuse cannot change its target.
exec "$PLUGIN_DIR/scripts/v2.sh" navigate --window "${1:-}" --client "${2:-}"
