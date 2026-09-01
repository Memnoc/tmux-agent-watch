#!/usr/bin/env bash

set -eu

ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
SOCKET="agent-watch-v2-$$"
TMP_DIR="$(mktemp -d)"

cargo build --offline --manifest-path "$ROOT/Cargo.toml" >/dev/null

cleanup() {
  tmux -L "$SOCKET" kill-server 2>/dev/null || true
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

tmux -L "$SOCKET" -f /dev/null new-session -d -s v2
tmux -L "$SOCKET" set-option -g @agent-watch-hud off
tmux -L "$SOCKET" set-option -g @agent-watch-sidebar off
tmux -L "$SOCKET" run-shell "$ROOT/tmux-agent-watch.tmux"

binding="$(tmux -L "$SOCKET" list-keys -T prefix | awk '$4 == "P" && /scripts\/v2.sh cockpit/')"
[ -n "$binding" ] || {
  printf 'not ok: v2 cockpit binding was not installed by default\n'
  exit 1
}
printf 'ok: v2 cockpit is the default on the existing cockpit key\n'

tmux -L "$SOCKET" set-option -g @agent-watch-v2 off
tmux -L "$SOCKET" run-shell "$ROOT/tmux-agent-watch.tmux"
binding="$(tmux -L "$SOCKET" list-keys -T prefix | awk '$4 == "P" && /scripts\/cockpit.sh/')"
[ -n "$binding" ] || {
  printf 'not ok: disabling v2 did not restore the v1 cockpit\n'
  exit 1
}
printf 'ok: v1 cockpit remains available as an explicit fallback\n'

fake_binary="$TMP_DIR/tmux-agent-watch"
apply_theme="$TMP_DIR/theme"
sed "s|OUTPUT_PATH|$apply_theme|" > "$fake_binary" <<'SCRIPT'
#!/usr/bin/env bash
printf '%s\n' "$*" > 'OUTPUT_PATH'
SCRIPT
chmod +x "$fake_binary"
tmux -L "$SOCKET" set-option -g @agent-watch-theme dawn
socket_path="$(tmux -L "$SOCKET" display-message -p '#{socket_path}')"
server_pid="$(tmux -L "$SOCKET" display-message -p '#{pid}')"
TMUX="$socket_path,$server_pid,0" AGENT_WATCH_V2_BIN="$fake_binary" "$ROOT/scripts/v2.sh" cockpit
[ "$(cat "$apply_theme")" = 'cockpit --theme dawn' ] || {
  printf 'not ok: v2 launcher did not forward the selected theme\n'
  exit 1
}
printf 'ok: v2 launcher forwards the Rose Pine theme variant\n'

before="$(find "$TMP_DIR" -mindepth 1 -maxdepth 1 -printf '%f\n' | sort)"
TMUX="$socket_path,$server_pid,0" AGENT_WATCH_V2_BIN="$fake_binary" "$ROOT/scripts/v2.sh" status
after="$(find "$TMP_DIR" -mindepth 1 -maxdepth 1 -printf '%f\n' | sort)"
[ "$before" = "$after" ] || {
  printf 'not ok: launcher created unexpected persistent state\n'
  exit 1
}
printf 'ok: v2 launcher creates no project-controlled state\n'

real_binary="$ROOT/target/debug/tmux-agent-watch"
[ -x "$real_binary" ] || {
  printf 'not ok: Rust debug binary is unavailable for lifecycle integration\n'
  exit 1
}
watcher_pid="$(tmux -L "$SOCKET" show-option -gqv @agent_watch_watcher_pid 2>/dev/null || true)"
[ -z "$watcher_pid" ] || kill "$watcher_pid" 2>/dev/null || true
tmux -L "$SOCKET" set-option -gq @agent_watch_watcher_pid ''
tmux -L "$SOCKET" set-option -g @agent-watch-v2 on
ln -s "$(command -v sleep)" "$TMP_DIR/codex"
tmux -L "$SOCKET" new-window -d -t v2 -n agent "$TMP_DIR/codex 30"
agent_pane="$(tmux -L "$SOCKET" list-panes -t v2:agent -F '#{pane_id}' | head -n 1)"
agent_window="$(tmux -L "$SOCKET" display-message -p -t "$agent_pane" '#{window_id}')"
tmux -L "$SOCKET" set-option -wq -t "$agent_window" @agent_watch_message 'sensitive legacy summary'
TMUX="$socket_path,$server_pid,0" AGENT_WATCH_V2_BIN="$real_binary" "$ROOT/scripts/v2.sh" scan
state="$(tmux -L "$SOCKET" show-option -wqv -t "$agent_window" @agent_watch_state)"
message="$(tmux -L "$SOCKET" show-option -wqv -t "$agent_window" @agent_watch_message)"
[ "$state" = working ] && [ -z "$message" ] || {
  printf 'not ok: v2 scan did not classify the process and erase legacy content\n'
  exit 1
}
printf 'ok: v2 scan classifies process identity and erases legacy content\n'

printf '%s' '{"prompt":"private customer material"}' |
  TMUX="$socket_path,$server_pid,0" TMUX_PANE="$agent_pane" AGENT_WATCH_V2_BIN="$real_binary" \
  "$ROOT/scripts/codex-hook.sh" permissionRequest
state="$(tmux -L "$SOCKET" show-option -wqv -t "$agent_window" @agent_watch_state)"
message="$(tmux -L "$SOCKET" show-option -wqv -t "$agent_window" @agent_watch_message)"
[ -z "$message" ] || {
  printf 'not ok: v2 hook retained payload content or mapped the event incorrectly\n'
  exit 1
}
printf 'ok: v2 hooks ignore payload content and publish fixed lifecycle state\n'

after_hook="$(tmux -L "$SOCKET" show-options -wv -t "$agent_window" | grep -F 'private customer material' || true)"
[ -z "$after_hook" ] || {
  printf 'not ok: private hook payload reached tmux state\n'
  exit 1
}
printf 'ok: hook payload is absent from all tmux window options\n'

tmux -L "$SOCKET" set-option -wq -t "$agent_window" @agent_watch_state needs_input

fleet="$(TMUX="$socket_path,$server_pid,0" AGENT_WATCH_V2_BIN="$real_binary" \
  "$ROOT/scripts/v2.sh" hud fleet v2 "$agent_window" moon)"
printf '%s' "$fleet" | grep -Fq '1 agents' || {
  printf 'not ok: v2 HUD did not project the live fleet\n'
  exit 1
}
printf '%s' "$fleet" | grep -Fq '1 waiting' || {
  printf 'not ok: v2 HUD did not project the waiting state\n'
  exit 1
}
printf 'ok: v2 HUD projects the unified workspace model\n'

sidebar="$(TMUX="$socket_path,$server_pid,0" AGENT_WATCH_V2_BIN="$real_binary" \
  "$ROOT/scripts/v2.sh" sidebar v2 "$agent_window" --expanded --theme moon)"
frame="${sidebar%$'\034'*}"
click_map="${sidebar##*$'\034'}"
printf '%s' "$frame" | grep -Fq 'WAITING' || {
  printf 'not ok: v2 sidebar did not render the fixed lifecycle label\n'
  exit 1
}
printf '%s' "$click_map" | grep -Fq "=$agent_window;" || {
  printf 'not ok: v2 sidebar did not provide a click mapping\n'
  exit 1
}
printf 'ok: v2 sidebar projects fixed metadata with click navigation\n'

tmux -L "$SOCKET" set-option -g @agent-watch-redact-labels on
redacted="$(TMUX="$socket_path,$server_pid,0" AGENT_WATCH_V2_BIN="$real_binary" \
  "$ROOT/scripts/v2.sh" sidebar v2 "$agent_window" --expanded --theme moon)"
redacted_frame="${redacted%$'\034'*}"
redacted_map="${redacted##*$'\034'}"
if printf '%s' "$redacted_frame" | grep -Eq '(^|[^[:alpha:]])agent([^[:alpha:]]|$)|work/'; then
  printf 'not ok: redacted sidebar exposed a workspace label\n'
  exit 1
fi
printf '%s' "$redacted_frame" | grep -Fq 'Workspace' &&
  printf '%s' "$redacted_map" | grep -Fq "=$agent_window;" || {
  printf 'not ok: redacted sidebar lost its private label or navigation map\n'
  exit 1
}
tmux -L "$SOCKET" set-option -g @agent-watch-redact-labels off
printf 'ok: display redaction hides labels without breaking navigation\n'

repo="$TMP_DIR/repo"
worktree_root="$TMP_DIR/worktrees"
git init -q "$repo"
git -C "$repo" config user.name 'Agent Watch Test'
git -C "$repo" config user.email 'agent-watch@example.invalid'
touch "$repo/README.md"
git -C "$repo" add README.md
git -C "$repo" commit -qm initial
git -C "$repo" branch -M main
created="$(TMUX="$socket_path,$server_pid,0" AGENT_WATCH_V2_BIN="$real_binary" \
  AGENT_WATCH_WORKTREE_ROOT="$worktree_root" "$ROOT/scripts/worktree-new.sh" \
  --repo "$repo" work/privacy "$TMP_DIR/codex" 30)"
[ "$created" = "$worktree_root/work-privacy" ] &&
  [ "$(git -C "$created" branch --show-current)" = work/privacy ] || {
  printf 'not ok: v2 start did not create the expected linked worktree\n'
  exit 1
}
created_window="$(tmux -L "$SOCKET" display-message -p -t v2:work-privacy '#{window_id}')"
[ "$(tmux -L "$SOCKET" show-option -wqv -t "$created_window" @agent_watch_message)" = '' ] || {
  printf 'not ok: v2 start created content-bearing tmux state\n'
  exit 1
}
printf 'ok: v2 start creates an isolated workspace without content state\n'

privacy_task='rotate private customer token 9f47c2'
privacy_pane="$(tmux -L "$SOCKET" new-window -d -P -F '#{pane_id}' -t v2: -n privacy cat)"
printf '%s' "$privacy_task" | TMUX="$socket_path,$server_pid,0" \
  "$real_binary" workspace deliver-task "$privacy_pane"
sleep 0.2
captured="$(tmux -L "$SOCKET" capture-pane -p -t "$privacy_pane")"
printf '%s' "$captured" | grep -Fq "$privacy_task" || {
  printf 'not ok: transient task did not reach the selected agent pane\n'
  exit 1
}
if tmux -L "$SOCKET" show-options -g -w -v 2>/dev/null | grep -Fq "$privacy_task" ||
  tmux -L "$SOCKET" list-buffers -F '#{buffer_sample}' 2>/dev/null | grep -Fq "$privacy_task" ||
  tmux -L "$SOCKET" show-environment -g 2>/dev/null | grep -Fq "$privacy_task" ||
  ps -eo args= | grep -F "$privacy_task" | grep -Fv grep >/dev/null; then
  printf 'not ok: transient task escaped into supervisor state or process metadata\n'
  exit 1
fi
printf 'ok: task delivery is transient at the CLI-to-tmux outer seam\n'

printf dirty >> "$created/README.md"
if (cd "$created" && printf 'y\n' | TMUX="$socket_path,$server_pid,0" \
  AGENT_WATCH_V2_BIN="$real_binary" "$ROOT/scripts/worktree-finish.sh") >/dev/null 2>&1; then
  printf 'not ok: v2 finish removed a dirty worktree\n'
  exit 1
fi
[ -d "$created" ] || { printf 'not ok: dirty worktree was lost\n'; exit 1; }
git -C "$created" restore README.md
removed="$(cd "$created" && printf 'y\n' | TMUX="$socket_path,$server_pid,0" \
  AGENT_WATCH_V2_BIN="$real_binary" "$ROOT/scripts/worktree-finish.sh")"
[ "$removed" = "$created" ] && [ ! -e "$created" ] || {
  printf 'not ok: v2 finish did not remove the eligible worktree\n'
  exit 1
}
git -C "$repo" show-ref --verify --quiet refs/heads/work/privacy || {
  printf 'not ok: v2 finish deleted the retained branch\n'
  exit 1
}
if tmux -L "$SOCKET" list-windows -a -F '#{window_id}' | grep -Fqx "$created_window"; then
  printf 'not ok: v2 finish left the removed workspace window open\n'
  exit 1
fi
printf 'ok: v2 finish enforces safety, retains the branch, and closes the window\n'
