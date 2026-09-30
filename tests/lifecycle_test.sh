#!/usr/bin/env bash
# Exercise window lifecycle scanning with mixed agent and ordinary panes.
set -eu
ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
SOCKET="drudwyn-lifecycle-$$"
TMP_DIR="$(mktemp -d)"
cleanup() {
  tmux -L "$SOCKET" kill-server 2>/dev/null || true
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT
cargo build --offline --manifest-path "$ROOT/Cargo.toml" >/dev/null
binary="$ROOT/target/debug/tmux-drudwyn"
ln -s "$(command -v sleep)" "$TMP_DIR/codex"
ln -s "$(command -v sleep)" "$TMP_DIR/nvim"
tmux -L "$SOCKET" -f /dev/null new-session -d -s lifecycle -n AOC-TS "$TMP_DIR/codex 300"
agent_pane="$(tmux -L "$SOCKET" display-message -p -t lifecycle:0 '#{pane_id}')"
window="$(tmux -L "$SOCKET" display-message -p -t lifecycle:0 '#{window_id}')"
helper_pane="$(tmux -L "$SOCKET" split-window -d -P -F '#{pane_id}' -t "$agent_pane" "$TMP_DIR/nvim 300")"
socket_path="$(tmux -L "$SOCKET" display-message -p '#{socket_path}')"
export TMUX="$socket_path,0,0"
tmux set-option -g @drudwyn-icon-mode safe

# Every scan must keep the live agent in the right-hand status cluster.
for scan in 1 2 3; do
  "$binary" scan
  bar="$(bash "$ROOT/scripts/status-bar.sh" lifecycle "$window" 177)"
  right="${bar##*'#[align=right]'}"
  if ! printf '%s' "$right" | grep -Fq AOC-TS; then
    printf 'not ok: AOC-TS disappeared from the agent cluster on scan %s\n' "$scan"
    exit 1
  fi
done
printf 'ok: an ordinary split pane cannot remove a live agent from the status bar\n'

# Match AOC-TS's actual order: editor first, agent second. A transient clear
# followed by reclassification also resets the lifecycle timestamp.
tmux swap-pane -s "$agent_pane" -t "$helper_pane"
tmux set-option -wq -t "$window" @drudwyn_since 100
for scan in 1 2 3; do
  "$binary" scan
  [ "$(tmux show-option -wqv -t "$window" @drudwyn_since)" = 100 ] || {
    printf 'not ok: editor-first scanning cleared and recreated the agent state\n'
    exit 1
  }
done
printf 'ok: editor-first scanning preserves the agent state without transient clears\n'

TMUX_PANE="$agent_pane" "$binary" hook codex permissionRequest
attention_since="$(tmux show-option -wqv -t "$window" @drudwyn_attention_since)"
for scan in 1 2 3; do
  "$binary" scan
  [ "$(tmux show-option -wqv -t "$window" @drudwyn_state)" = needs_input ] &&
    [ "$(tmux show-option -wqv -t "$window" @drudwyn_source)" = hook ] &&
    [ "$(tmux show-option -wqv -t "$window" @drudwyn_attention_since)" = "$attention_since" ] || {
    printf 'not ok: scanning overwrote attention from the agent hook\n'
    exit 1
  }
done
printf 'ok: split-pane scans preserve attention from lifecycle hooks\n'

tmux set-option -wq -t "$window" @drudwyn_source process
tmux kill-pane -t "$agent_pane"
"$binary" scan
[ -z "$(tmux show-option -wqv -t "$window" @drudwyn_state)" ] &&
  [ -z "$(tmux show-option -wqv -t "$window" @drudwyn_agent)" ] || {
  printf 'not ok: scanning retained an agent after its last pane closed\n'
  exit 1
}
printf 'ok: process-derived state clears after the last agent pane closes\n'
