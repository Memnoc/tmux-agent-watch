#!/usr/bin/env bash
set -eu
ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
fixture="$(mktemp -d)"
trap 'rm -rf "$fixture"' EXIT
mkdir -p "$fixture/source"
cp -R "$ROOT/scripts" "$ROOT/assets" "$ROOT/tmux-drudwyn.tmux" "$fixture/source/"
DRUDWYN_RUNTIME_BINARY="$ROOT/target/debug/tmux-drudwyn" bash "$fixture/source/scripts/install-runtime.sh" "$fixture/runtime" >/dev/null
selected="$(readlink "$fixture/runtime/current")"
# Model the development branch switch removing every feature runtime file.
rm -rf "$fixture/source"
"$fixture/runtime/current/scripts/v2.sh" cockpit --help >/dev/null
"$fixture/runtime/current/scripts/v2.sh" navigator --help >/dev/null
"$fixture/runtime/current/scripts/v2.sh" sessions --help >/dev/null
[ -x "$fixture/runtime/current/scripts/navigation-popup.sh" ]
[ -x "$fixture/runtime/current/scripts/codex-hook.sh" ]
if DRUDWYN_RUNTIME_BINARY="$fixture/missing" bash "$ROOT/scripts/install-runtime.sh" "$fixture/runtime" >/dev/null 2>&1; then
  printf 'not ok: missing binary accepted\n'; exit 1
fi
[ "$(readlink "$fixture/runtime/current")" = "$selected" ]
printf 'ok: runtime survives source removal; failed replacement retains installed snapshot\n'
