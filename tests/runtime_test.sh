#!/usr/bin/env bash
set -eu
ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
fixture="$(mktemp -d)"
socket="$fixture/tmux.sock"
trap 'tmux -S "$socket" kill-server 2>/dev/null || true; rm -rf "$fixture"' EXIT
mkdir -p "$fixture/source"
cp -R "$ROOT/scripts" "$ROOT/assets" "$ROOT/tmux-drudwyn.tmux" "$fixture/source/"
DRUDWYN_RUNTIME_BINARY="$ROOT/target/debug/tmux-drudwyn" bash "$fixture/source/scripts/install-runtime.sh" "$fixture/runtime" >/dev/null
selected="$(readlink "$fixture/runtime/current")"
# A live loader migration replaces only Drudwyn hooks and its own observer.
tmux -S "$socket" -f /dev/null new-session -d -s runtime
export TMUX="$socket,0,0"
unset TMUX_PANE DRUDWYN_CLIENT
export DRUDWYN_V2_BIN="$ROOT/target/debug/tmux-drudwyn"
bash "$fixture/source/tmux-drudwyn.tmux"
old_watcher="$(tmux show-option -gqv @drudwyn_watcher_pid)"
tmux set-hook -g 'after-new-window[42]' 'display-message unrelated-plugin'
bash "$fixture/runtime/current/tmux-drudwyn.tmux"
new_watcher="$(tmux show-option -gqv @drudwyn_watcher_pid)"
[ "$old_watcher" != "$new_watcher" ]
hooks="$(tmux show-hooks -g after-new-window)"
[[ "$hooks" != *"$fixture/source"* ]]
[[ "$hooks" == *"unrelated-plugin"* ]]
[[ "$hooks" == *"$fixture/runtime/current"* ]]
printf 'ok: runtime switch replaces Drudwyn hooks and observer, preserving other plugins\n'
unset DRUDWYN_V2_BIN
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
