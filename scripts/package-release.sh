#!/usr/bin/env bash

set -eu

ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
target="${1:-}"
binary="${2:-}"
output="${3:-$ROOT/dist}"

[ -n "$target" ] && [ -n "$binary" ] || {
  printf 'usage: %s TARGET BINARY [OUTPUT_DIRECTORY]\n' "$0" >&2
  exit 2
}
[ -x "$binary" ] || {
  printf 'tmux-agent-watch: release binary is missing or not executable: %s\n' "$binary" >&2
  exit 1
}

archive="tmux-agent-watch-${target}.tar.gz"
stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
mkdir -p "$stage/tmux-agent-watch" "$output"
install -m 0755 "$binary" "$stage/tmux-agent-watch/tmux-agent-watch"
install -m 0644 "$ROOT/LICENSE" "$stage/tmux-agent-watch/LICENSE"
install -m 0644 "$ROOT/README.md" "$stage/tmux-agent-watch/README.md"
install -m 0644 "$ROOT/docs/privacy.md" "$stage/tmux-agent-watch/PRIVACY.md"
tar -C "$stage" -czf "$output/$archive" tmux-agent-watch
printf '%s\n' "$output/$archive"
