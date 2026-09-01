#!/usr/bin/env bash

set -eu

ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
SOCKET="agent-watch-cockpit-test-$$"
TMP_DIR="$(mktemp -d)"
REPO="$TMP_DIR/example"

cleanup() {
  tmux -L "$SOCKET" kill-server 2>/dev/null || true
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

git init -q "$REPO"
git -C "$REPO" config user.name 'Cockpit Test'
git -C "$REPO" config user.email 'cockpit@example.invalid'
touch "$REPO/README.md"
git -C "$REPO" add README.md
git -C "$REPO" commit -qm 'initial commit'
git -C "$REPO" branch -M main
cp "$(command -v cat)" "$TMP_DIR/codex"

tmux -L "$SOCKET" -f /dev/null new-session -d -s cockpit -x 110 -y 32 -c "$REPO"
tmux -L "$SOCKET" set-option -g @agent-watch-v2 off
socket_path="$(tmux -L "$SOCKET" display-message -p '#{socket_path}')"
server_pid="$(tmux -L "$SOCKET" display-message -p '#{pid}')"
tmux -L "$SOCKET" set-environment -g PATH "$TMP_DIR:$PATH"
tmux -L "$SOCKET" run-shell "$ROOT/tmux-agent-watch.tmux"
cockpit_pane="$(tmux -L "$SOCKET" new-window -d -P -F '#{pane_id}' -t cockpit: -c "$REPO" \
  "TMUX='$socket_path,$server_pid,0' '$ROOT/scripts/cockpit.sh'")"
sleep 1

frame="$(tmux -L "$SOCKET" capture-pane -p -t "$cockpit_pane")"
printf '%s\n' "$frame" | grep -Fq 'WORKSPACE COMMAND CENTER' || {
  printf 'not ok: cockpit title is not rendered\n'; exit 1;
}
printf '%s\n' "$frame" | grep -Fq 'Start a quick win' || {
  printf 'not ok: cockpit does not expose its primary action\n'; exit 1;
}
printf '%s\n' "$frame" | grep -Fq 'Current repo: example' || {
  printf 'not ok: cockpit does not show live repository context\n'; exit 1;
}
printf 'ok: cockpit renders an action-first view with live repository context\n'

binding="$(tmux -L "$SOCKET" list-keys -T prefix | grep ' P .*scripts/cockpit.sh' || true)"
[ -n "$binding" ] || { printf 'not ok: cockpit binding missing\n'; exit 1; }
printf 'ok: cockpit is bound to prefix + P by default\n'

tmux -L "$SOCKET" send-keys -t "$cockpit_pane" 1
sleep 0.2
frame="$(tmux -L "$SOCKET" capture-pane -p -t "$cockpit_pane")"
printf '%s\n' "$frame" | grep -Fq 'START A QUICK WIN' || {
  printf 'not ok: Start does not open its guided task flow\n'; exit 1
}
tmux -L "$SOCKET" send-keys -l -t "$cockpit_pane" 'Add version command'
tmux -L "$SOCKET" send-keys -t "$cockpit_pane" Enter
sleep 0.2
frame="$(tmux -L "$SOCKET" capture-pane -p -t "$cockpit_pane")"
printf '%s\n' "$frame" | grep -Fq 'quick-win/add-version-command' || {
  printf 'not ok: Start does not show its generated branch\n'; exit 1
}
printf '%s\n' "$frame" | grep -Fq '[1] Codex' || {
  printf 'not ok: Start does not offer agent selection\n'; exit 1
}
tmux -L "$SOCKET" send-keys -t "$cockpit_pane" 1
sleep 0.2
frame="$(tmux -L "$SOCKET" capture-pane -p -t "$cockpit_pane")"
printf '%s\n' "$frame" | grep -Fq 'Enter create · Esc cancel' || {
  printf 'not ok: Start does not confirm the generated workspace\n'; exit 1
}
tmux -L "$SOCKET" send-keys -t "$cockpit_pane" Enter
sleep 1
created="$TMP_DIR/example-worktrees/quick-win-add-version-command"
[ -d "$created" ] && [ "$(git -C "$created" branch --show-current)" = quick-win/add-version-command ] || {
  printf 'not ok: Start did not create its linked worktree\n'; exit 1
}
created_window="$(tmux -L "$SOCKET" list-windows -a -F '#{window_id}|#{@agent_watch_worktree}' |
  awk -F '|' -v path="$created" '$2 == path { print $1; exit }')"
[ -n "$created_window" ] || { printf 'not ok: Start did not create an agent window\n'; exit 1; }
printf 'ok: Start creates a confirmed branch worktree and chosen agent window\n'

tmux -L "$SOCKET" set-option -wq -t "$created_window" @agent_watch_state done
tmux -L "$SOCKET" set-option -wq -t "$created_window" @agent_watch_message 'Version command ready for review'
tmux -L "$SOCKET" set-option -wq -t "$created_window" @agent_watch_source hook
cockpit_pane="$(tmux -L "$SOCKET" new-window -d -P -F '#{pane_id}' -t cockpit: -c "$REPO" \
  "TMUX='$socket_path,$server_pid,0' '$ROOT/scripts/cockpit.sh'")"
sleep 0.2
tmux -L "$SOCKET" send-keys -t "$cockpit_pane" 2
sleep 0.2
frame="$(tmux -L "$SOCKET" capture-pane -p -t "$cockpit_pane")"
printf '%s\n' "$frame" | grep -Fq 'REVIEW READY WORK' &&
  printf '%s\n' "$frame" | grep -Fq 'quick-win/add-version-command' &&
  printf '%s\n' "$frame" | grep -Fq 'Version command ready for review' || {
  printf 'not ok: Review does not list live attention workspaces\n'; exit 1
}
tmux -L "$SOCKET" send-keys -t "$cockpit_pane" 1
sleep 0.5
current_window="$(tmux -L "$SOCKET" display-message -p -t cockpit: '#{window_id}')"
[ "$current_window" = "$created_window" ] || {
  printf 'not ok: Review did not jump to the selected workspace\n'; exit 1
}
printf 'ok: Review filters attention workspaces and jumps to the selected workspace\n'

cockpit_pane="$(tmux -L "$SOCKET" new-window -d -P -F '#{pane_id}' -t cockpit: -c "$REPO" \
  "TMUX='$socket_path,$server_pid,0' '$ROOT/scripts/cockpit.sh'")"
sleep 0.2
tmux -L "$SOCKET" send-keys -t "$cockpit_pane" 3
sleep 0.2
frame="$(tmux -L "$SOCKET" capture-pane -p -t "$cockpit_pane")"
printf '%s\n' "$frame" | grep -Fq 'JUMP TO A WORKSPACE' &&
  printf '%s\n' "$frame" | grep -Fq 'quick-win/add-version-command' || {
  printf 'not ok: Jump does not list live agent workspaces\n'; exit 1
}
tmux -L "$SOCKET" send-keys -t "$cockpit_pane" 1
sleep 0.5
current_window="$(tmux -L "$SOCKET" display-message -p -t cockpit: '#{window_id}')"
[ "$current_window" = "$created_window" ] || {
  printf 'not ok: Jump selected %s instead of %s\n' "$current_window" "$created_window"; exit 1
}
printf 'ok: Jump selects the chosen live agent workspace\n'

cockpit_pane="$(tmux -L "$SOCKET" new-window -d -P -F '#{pane_id}' -t cockpit: -c "$REPO" \
  "TMUX='$socket_path,$server_pid,0' '$ROOT/scripts/cockpit.sh'")"
sleep 0.2
tmux -L "$SOCKET" send-keys -t "$cockpit_pane" 4
sleep 0.2
frame="$(tmux -L "$SOCKET" capture-pane -p -t "$cockpit_pane")"
printf '%s\n' "$frame" | grep -Fq 'FINISH MERGED WORK' &&
  printf '%s\n' "$frame" | grep -Fq 'quick-win/add-version-command' || {
  printf 'not ok: Finish does not list the clean merged worktree\n'; exit 1
}
tmux -L "$SOCKET" send-keys -t "$cockpit_pane" 1
sleep 0.2
frame="$(tmux -L "$SOCKET" capture-pane -p -t "$cockpit_pane")"
printf '%s\n' "$frame" | grep -Fq 'The branch will be retained.' &&
  printf '%s\n' "$frame" | grep -Fq '[y/N]' || {
  printf 'not ok: Finish does not preserve its confirmation safeguard\n'; exit 1
}
tmux -L "$SOCKET" send-keys -t "$cockpit_pane" y Enter
sleep 1
[ ! -e "$created" ] || { printf 'not ok: Finish did not remove the linked worktree\n'; exit 1; }
git -C "$REPO" show-ref --verify --quiet refs/heads/quick-win/add-version-command || {
  printf 'not ok: Finish deleted the retained branch\n'; exit 1
}
printf 'ok: Finish lists only eligible work and removes it with confirmation\n'

cockpit_pane="$(tmux -L "$SOCKET" new-window -d -P -F '#{pane_id}' -t cockpit: -c "$REPO" \
  "TMUX='$socket_path,$server_pid,0' '$ROOT/scripts/cockpit.sh'")"
sleep 0.2
tmux -L "$SOCKET" send-keys -t "$cockpit_pane" 4
sleep 0.2
frame="$(tmux -L "$SOCKET" capture-pane -p -t "$cockpit_pane")"
printf '%s\n' "$frame" | grep -Fq 'No clean merged worktrees are ready to finish.' || {
  printf 'not ok: Finish does not show an inline empty-state message\n'; exit 1
}
if ! tmux -L "$SOCKET" list-panes -a -F '#{pane_id}' | grep -Fxq "$cockpit_pane"; then
  printf 'not ok: cockpit disappeared after an action error\n'; exit 1
fi
printf 'ok: action errors remain visible inside the cockpit\n'
tmux -L "$SOCKET" send-keys -t "$cockpit_pane" x
sleep 0.2
tmux -L "$SOCKET" send-keys -t "$cockpit_pane" q
sleep 1
if tmux -L "$SOCKET" list-panes -a -F '#{pane_id}' | grep -Fxq "$cockpit_pane"; then
  printf 'not ok: q did not close the cockpit\n'; exit 1
fi
printf 'ok: cockpit closes with q\n'
