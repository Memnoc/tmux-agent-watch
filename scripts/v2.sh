#!/usr/bin/env bash

set -eu

PLUGIN_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
binary="${AGENT_WATCH_V2_BIN:-}"

if [ -z "$binary" ] && [ -x "$PLUGIN_DIR/target/release/tmux-agent-watch" ]; then
  binary="$PLUGIN_DIR/target/release/tmux-agent-watch"
fi
if [ -z "$binary" ]; then
  binary="$(command -v tmux-agent-watch 2>/dev/null || true)"
fi
if [ -z "$binary" ] || [ ! -x "$binary" ]; then
  printf '%s\n' 'tmux-agent-watch v2 binary not found.' >&2
  printf '%s\n' 'Install it with cargo install --path . or build target/release/tmux-agent-watch.' >&2
  exit 127
fi

case "${1:-}" in
  cockpit)
    theme="$(tmux show-option -gqv @agent-watch-theme 2>/dev/null || true)"
    theme="${theme:-moon}"
    exec "$binary" cockpit --theme "$theme"
    ;;
  navigator)
    theme="$(tmux show-option -gqv @agent-watch-theme 2>/dev/null || true)"
    exec "$binary" navigator --theme "${theme:-moon}"
    ;;
  status) exec "$binary" status ;;
  scan) exec "$binary" scan ;;
  hud) exec "$binary" hud "$2" "$3" "$4" --theme "${5:-moon}" ;;
  sidebar) exec "$binary" sidebar "${@:2}" ;;
  workspace) shift; exec "$binary" workspace "$@" ;;
  hook)
    [ "$#" -ge 3 ] || { printf 'usage: %s hook AGENT EVENT\n' "$0" >&2; exit 2; }
    exec "$binary" hook "$2" "$3"
    ;;
  *)
    printf 'usage: %s cockpit|navigator|status|scan|hook AGENT EVENT\n' "$0" >&2
    exit 2
    ;;
esac
