#!/usr/bin/env bash

set -eu

directory="${1:-dist}"
[ -d "$directory" ] || {
  printf 'tmux-agent-watch: release directory not found: %s\n' "$directory" >&2
  exit 1
}

temporary="$(mktemp -d)"
trap 'rm -rf "$temporary"' EXIT

for target in \
  x86_64-unknown-linux-gnu \
  aarch64-unknown-linux-gnu \
  x86_64-apple-darwin \
  aarch64-apple-darwin
do
  archive="$directory/tmux-agent-watch-${target}.tar.gz"
  [ -f "$archive" ] || {
    printf 'tmux-agent-watch: release archive is missing: %s\n' "${archive##*/}" >&2
    exit 1
  }
  tar -tzf "$archive" > "$temporary/$target.list"
  grep -Fxq 'tmux-agent-watch/tmux-agent-watch' "$temporary/$target.list" || {
    printf 'tmux-agent-watch: binary is missing from %s\n' "${archive##*/}" >&2
    exit 1
  }
  grep -Fxq 'tmux-agent-watch/PRIVACY.md' "$temporary/$target.list" || {
    printf 'tmux-agent-watch: privacy manifest is missing from %s\n' "${archive##*/}" >&2
    exit 1
  }
done

[ "$(find "$directory" -maxdepth 1 -name '*.tar.gz' | wc -l)" -eq 4 ] || {
  printf 'tmux-agent-watch: release bundle must contain exactly four archives\n' >&2
  exit 1
}
(
  cd "$directory"
  sha256sum *.tar.gz | sort -k2 > SHA256SUMS
  sha256sum --check SHA256SUMS
)
