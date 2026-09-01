#!/usr/bin/env bash

set -eu

ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
SOCKET="agent-watch-worktree-test-$$"
TMP_DIR="$(mktemp -d)"
REPO="$TMP_DIR/example"
WORKTREES="$TMP_DIR/worktrees"

cleanup() {
  tmux -L "$SOCKET" kill-server 2>/dev/null || true
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

git init -q "$REPO"
git -C "$REPO" config user.name 'Agent Watch Test'
git -C "$REPO" config user.email 'agent-watch@example.invalid'
touch "$REPO/README.md"
git -C "$REPO" add README.md
git -C "$REPO" commit -qm 'initial commit'
git -C "$REPO" branch -M main

ln -s "$(command -v sleep)" "$TMP_DIR/codex"
tmux -L "$SOCKET" -f /dev/null new-session -d -s agents -c "$REPO"
socket_path="$(tmux -L "$SOCKET" display-message -p '#{socket_path}')"
server_pid="$(tmux -L "$SOCKET" display-message -p '#{pid}')"

created="$({
  cd "$TMP_DIR"
  TMUX="$socket_path,$server_pid,0" \
    AGENT_WATCH_WORKTREE_ROOT="$WORKTREES" \
    "$ROOT/scripts/worktree-new.sh" --repo "$REPO" feature/auth "$TMP_DIR/codex" 30
})"

[ "$created" = "$WORKTREES/feature-auth" ] || {
  printf 'not ok: launcher returned unexpected path %s\n' "$created"
  exit 1
}
[ "$(git -C "$created" branch --show-current)" = feature/auth ] || {
  printf 'not ok: worktree is not on feature/auth\n'
  exit 1
}
git -C "$REPO" worktree list --porcelain | grep -Fqx "worktree $created" || {
  printf 'not ok: Git does not list created worktree\n'
  exit 1
}
created_window="$(tmux -L "$SOCKET" display-message -p -t agents:feature-auth '#{window_id}')"
window_path="$(tmux -L "$SOCKET" display-message -p -t "$created_window" '#{pane_current_path}')"
[ "$window_path" = "$created" ] || {
  printf 'not ok: tmux window started in %s instead of %s\n' "$window_path" "$created"
  exit 1
}
[ "$(tmux -L "$SOCKET" show-option -wqv -t "$created_window" @agent_watch_branch)" = feature/auth ] || {
  printf 'not ok: tmux window does not expose its worktree branch\n'
  exit 1
}
[ "$(tmux -L "$SOCKET" show-option -wqv -t "$created_window" @agent_watch_worktree)" = "$created" ] || {
  printf 'not ok: tmux window does not expose its worktree path\n'
  exit 1
}
printf 'ok: launcher creates an isolated branch worktree and tmux window\n'

tmux -L "$SOCKET" set-option -wuq -t "$created_window" @agent_watch_branch
tmux -L "$SOCKET" set-option -wuq -t "$created_window" @agent_watch_worktree
tmux -L "$SOCKET" set-option -g @agent-watch-git-interval 0
TMUX="$socket_path,$server_pid,0" "$ROOT/scripts/scan.sh"
[ "$(tmux -L "$SOCKET" show-option -wqv -t "$created_window" @agent_watch_branch)" = feature/auth ] &&
  [ "$(tmux -L "$SOCKET" show-option -wqv -t "$created_window" @agent_watch_worktree)" = "$created" ] || {
  printf 'not ok: observer did not discover an existing linked worktree\n'
  exit 1
}
[ "$(tmux -L "$SOCKET" show-option -wqv -t "$created_window" @agent_watch_git_status)" = clean ] || {
  printf 'not ok: observer did not report a clean worktree\n'
  exit 1
}
printf 'dirty\n' >> "$created/README.md"
TMUX="$socket_path,$server_pid,0" "$ROOT/scripts/scan.sh"
[ "$(tmux -L "$SOCKET" show-option -wqv -t "$created_window" @agent_watch_git_status)" = dirty ] || {
  printf 'not ok: observer did not report a dirty worktree\n'
  exit 1
}
created_pane="$(tmux -L "$SOCKET" list-panes -t "$created_window" -F '#{pane_id}|#{@agent_watch_sidebar}' |
  awk -F '|' '$2 != 1 { print $1; exit }')"
TMUX="$socket_path,$server_pid,0" TMUX_PANE="$created_pane" "$ROOT/scripts/sidebar-resize.sh"
sleep 1
sidebar="$(tmux -L "$SOCKET" show-option -qv -t agents @agent_watch_sidebar_pane)"
sidebar_frame="$(tmux -L "$SOCKET" capture-pane -p -t "$sidebar")"
printf '%s\n' "$sidebar_frame" | grep -Fq '◆ WT feature/auth · DIRTY' || {
  printf 'not ok: expanded sidebar does not identify the dirty linked worktree\n'
  exit 1
}
TMUX="$socket_path,$server_pid,0" TMUX_PANE="$created_pane" "$ROOT/scripts/sidebar-resize.sh"
git -C "$created" restore README.md
printf 'ok: observer discovers external worktree metadata and dirty state\n'

tmux -L "$SOCKET" select-window -t agents:0
lazygit_path="$(
  cd "$created"
  TMUX="$socket_path,$server_pid,0" AGENT_WATCH_LAZYGIT="$(command -v pwd)" \
    "$ROOT/scripts/worktree-lazygit.sh"
)"
[ "$lazygit_path" = "$created" ] || {
  printf 'not ok: lazygit action opened in %s instead of %s\n' "$lazygit_path" "$created"
  exit 1
}
printf 'ok: lazygit action opens in the selected worktree\n'

if {
  cd "$REPO"
  TMUX="$socket_path,$server_pid,0" \
    AGENT_WATCH_WORKTREE_ROOT="$WORKTREES" \
    "$ROOT/scripts/worktree-new.sh" feature/auth "$TMP_DIR/codex" 30
} >/dev/null 2>&1; then
  printf 'not ok: duplicate launcher unexpectedly succeeded\n'
  exit 1
fi
[ "$(git -C "$created" branch --show-current)" = feature/auth ] || {
  printf 'not ok: duplicate launch disturbed the existing worktree\n'
  exit 1
}
printf 'ok: launcher rejects branch and path collisions\n'

printf 'dirty\n' >> "$created/README.md"
if {
  cd "$REPO"
  TMUX="$socket_path,$server_pid,0" \
    AGENT_WATCH_WORKTREE_ROOT="$WORKTREES" \
    "$ROOT/scripts/worktree-remove.sh" feature/auth
} >/dev/null 2>&1; then
  printf 'not ok: dirty worktree removal unexpectedly succeeded\n'
  exit 1
fi
[ -d "$created" ] || {
  printf 'not ok: dirty worktree was removed\n'
  exit 1
}
printf 'ok: cleanup refuses a dirty worktree\n'

git -C "$created" restore README.md
removed="$({
  cd "$created"
  printf 'y\n' | TMUX="$socket_path,$server_pid,0" \
    "$ROOT/scripts/worktree-finish.sh"
})"
[ "$removed" = "$created" ] || {
  printf 'not ok: cleanup returned unexpected path %s\n' "$removed"
  exit 1
}
[ ! -e "$created" ] || {
  printf 'not ok: clean worktree still exists\n'
  exit 1
}
git -C "$REPO" show-ref --verify --quiet refs/heads/feature/auth || {
  printf 'not ok: cleanup deleted the feature branch\n'
  exit 1
}
if tmux -L "$SOCKET" list-windows -a -F '#{window_id}' | grep -Fqx "$created_window"; then
  printf 'not ok: cleanup left the worktree window open\n'
  exit 1
fi
printf 'ok: cleanup removes a clean worktree, closes its window, and retains its branch\n'
