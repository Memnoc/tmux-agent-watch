#!/usr/bin/env bash

set -eu

PLUGIN_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
binary="${DRUDWYN_V2_BIN:-}"

if [ -z "$binary" ] && [ -x "$PLUGIN_DIR/target/release/tmux-drudwyn" ]; then
  binary="$PLUGIN_DIR/target/release/tmux-drudwyn"
fi
if [ -z "$binary" ]; then
  binary="$(command -v tmux-drudwyn 2>/dev/null || true)"
fi
if [ -z "$binary" ] || [ ! -x "$binary" ]; then
  printf '%s\n' 'tmux-drudwyn v2 binary not found.' >&2
  printf '%s\n' 'Install it with cargo install --path . or build target/release/tmux-drudwyn.' >&2
  exit 127
fi

case "${1:-}" in
  cockpit)
    theme="$(tmux show-option -gqv @drudwyn-theme 2>/dev/null || true)"
    theme="${theme:-rose-pine}"
    shift
    exec "$binary" cockpit --theme "$theme" "$@"
    ;;
  navigator)
    theme="$(tmux show-option -gqv @drudwyn-theme 2>/dev/null || true)"
    shift
    exec "$binary" navigator --theme "${theme:-rose-pine}" "$@"
    ;;
  sessions)
    theme="$(tmux show-option -gqv @drudwyn-theme 2>/dev/null || true)"
    shift
    exec "$binary" sessions --theme "${theme:-rose-pine}" "$@"
    ;;
  settings)
    theme="$(tmux show-option -gqv @drudwyn-theme 2>/dev/null || true)"
    exec "$binary" settings --theme "${theme:-rose-pine}"
    ;;
  status-bar|status-action) command="$1"; shift; exec "$binary" "$command" "$@" ;;
  navigate) shift; exec "$binary" navigate "$@" ;;
  status) exec "$binary" status ;;
  scan) exec "$binary" scan ;;
  hud) exec "$binary" hud "$2" "$3" "$4" --theme "${5:-rose-pine}" ;;
  sidebar) exec "$binary" sidebar "${@:2}" ;;
  workspace) shift; exec "$binary" workspace "$@" ;;
  hook)
    [ "$#" -ge 3 ] || { printf 'usage: %s hook AGENT EVENT\n' "$0" >&2; exit 2; }
    exec "$binary" hook "$2" "$3"
    ;;
  *)
    printf 'usage: %s cockpit|navigator|sessions|settings|status|scan|hook AGENT EVENT\n' "$0" >&2
    exit 2
    ;;
esac
