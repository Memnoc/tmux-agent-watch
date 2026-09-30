#!/usr/bin/env bash
set -eu
PLUGIN_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
# Errors replace the row and its ranges; no stale actionable output is retained.
if ! "$PLUGIN_DIR/scripts/v2.sh" status-bar --projection --session "${1:-}" --window "${2:-}" --width "${3:-120}" --row "${4:-tabs}" 2>/dev/null; then
  printf 'Status unavailable; refresh / open Cockpit'
fi
