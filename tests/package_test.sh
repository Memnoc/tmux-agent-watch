#!/usr/bin/env bash

set -eu

ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

cargo build --offline --manifest-path "$ROOT/Cargo.toml" >/dev/null
archive="$($ROOT/scripts/package-release.sh x86_64-unknown-linux-gnu \
  "$ROOT/target/debug/tmux-agent-watch" "$TMP_DIR/release")"
tar -tzf "$archive" | grep -Fxq 'tmux-agent-watch/tmux-agent-watch' || {
  printf 'not ok: release archive does not contain the binary\n'
  exit 1
}
tar -tzf "$archive" | grep -Fxq 'tmux-agent-watch/PRIVACY.md' || {
  printf 'not ok: release archive does not contain the privacy manifest\n'
  exit 1
}
(cd "$TMP_DIR/release" && sha256sum "${archive##*/}" > SHA256SUMS)

mkdir -p "$TMP_DIR/bin"
TMUX_AGENT_WATCH_BASE_URL="file://$TMP_DIR/release" \
  TMUX_AGENT_WATCH_INSTALL_DIR="$TMP_DIR/bin" "$ROOT/install.sh" >/dev/null
version="$(sed -n 's/^version = "\([^"]*\)"/\1/p' "$ROOT/Cargo.toml" | head -n 1)"
"$TMP_DIR/bin/tmux-agent-watch" --version | grep -Fq "tmux-agent-watch $version" || {
  printf 'not ok: installed release binary is not runnable\n'
  exit 1
}
printf 'tampered' >> "$archive"
if TMUX_AGENT_WATCH_BASE_URL="file://$TMP_DIR/release" \
  TMUX_AGENT_WATCH_INSTALL_DIR="$TMP_DIR/rejected" "$ROOT/install.sh" >/dev/null 2>&1; then
  printf 'not ok: installer accepted a tampered release archive\n'
  exit 1
fi
printf 'ok: release archive installs with verified checksum\n'

# Repackage after the deliberate installer-tamper check above.
"$ROOT/scripts/package-release.sh" x86_64-unknown-linux-gnu \
  "$ROOT/target/debug/tmux-agent-watch" "$TMP_DIR/release" >/dev/null
for target in \
  aarch64-unknown-linux-gnu \
  x86_64-apple-darwin \
  aarch64-apple-darwin
do
  cp "$TMP_DIR/release/tmux-agent-watch-x86_64-unknown-linux-gnu.tar.gz" \
    "$TMP_DIR/release/tmux-agent-watch-$target.tar.gz"
done
"$ROOT/scripts/verify-release-bundle.sh" "$TMP_DIR/release" >/dev/null
grep -Eq '^[0-9a-f]{64}  tmux-agent-watch-' "$TMP_DIR/release/SHA256SUMS" || {
  printf 'not ok: bundle checksums do not use parseable filenames\n'
  exit 1
}
if ! TMUX_AGENT_WATCH_BASE_URL="file://$TMP_DIR/release" \
  TMUX_AGENT_WATCH_INSTALL_DIR="$TMP_DIR/bundle-install" "$ROOT/install.sh" >/dev/null; then
  printf 'not ok: installer rejected checksums generated for the release bundle\n'
  exit 1
fi
printf 'ok: four-platform release bundle verifies without pipeline truncation\n'
