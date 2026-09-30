#!/usr/bin/env bash

set -eu

ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
SOCKET="drudwyn-v2-$$"
TMP_DIR="$(mktemp -d)"

cargo build --offline --manifest-path "$ROOT/Cargo.toml" >/dev/null

cleanup() {
  tmux -L "$SOCKET" kill-server 2>/dev/null || true
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

tmux -L "$SOCKET" -f /dev/null new-session -d -s v2
tmux -L "$SOCKET" set-option -g @drudwyn-hud off
tmux -L "$SOCKET" run-shell "$ROOT/tmux-drudwyn.tmux"

binding="$(tmux -L "$SOCKET" list-keys -T prefix | awk '$4 == "P" && /scripts\/navigation-popup.sh.* cockpit/')"
[ -n "$binding" ] || {
  printf 'not ok: v2 cockpit binding was not installed by default\n'
  exit 1
}
printf 'ok: v2 cockpit is the default on the existing cockpit key\n'
binding="$(tmux -L "$SOCKET" list-keys -T prefix | awk '$4 == "W" && /cockpit --start/')"
[ -n "$binding" ] || {
  printf 'not ok: worktree key bypasses the base selection form\n'; exit 1;
}
printf 'ok: worktree key opens the base selection form\n'


tmux -L "$SOCKET" set-option -g @drudwyn-v2 off
tmux -L "$SOCKET" run-shell "$ROOT/tmux-drudwyn.tmux"
binding="$(tmux -L "$SOCKET" list-keys -T prefix | awk '$4 == "P" && /scripts\/cockpit.sh/')"
[ -n "$binding" ] || {
  printf 'not ok: disabling v2 did not restore the v1 cockpit\n'
  exit 1
}
printf 'ok: v1 cockpit remains available as an explicit fallback\n'

fake_binary="$TMP_DIR/tmux-drudwyn"
apply_theme="$TMP_DIR/theme"
sed "s|OUTPUT_PATH|$apply_theme|" > "$fake_binary" <<'SCRIPT'
#!/usr/bin/env bash
printf '%s\n' "$*" > 'OUTPUT_PATH'
SCRIPT
chmod +x "$fake_binary"
tmux -L "$SOCKET" set-option -g @drudwyn-theme dawn
socket_path="$(tmux -L "$SOCKET" display-message -p '#{socket_path}')"
server_pid="$(tmux -L "$SOCKET" display-message -p '#{pid}')"
TMUX="$socket_path,$server_pid,0" DRUDWYN_V2_BIN="$fake_binary" "$ROOT/scripts/v2.sh" cockpit
[ "$(cat "$apply_theme")" = 'cockpit --theme dawn' ] || {
  printf 'not ok: v2 launcher did not forward the selected theme\n'
  exit 1
}
printf 'ok: v2 launcher forwards the Rose Pine theme variant\n'
TMUX="$socket_path,$server_pid,0" DRUDWYN_V2_BIN="$fake_binary" "$ROOT/scripts/v2.sh" settings
[ "$(cat "$apply_theme")" = 'settings --theme dawn' ] || {
  printf 'not ok: settings launcher did not forward the selected theme\n'
  exit 1
}
printf 'ok: settings launcher forwards the Rose Pine theme variant\n'
TMUX="$socket_path,$server_pid,0" DRUDWYN_V2_BIN="$fake_binary" "$ROOT/scripts/v2.sh" cockpit --start
[ "$(cat "$apply_theme")" = 'cockpit --theme dawn --start' ] || {
  printf 'not ok: v2 launcher dropped the start form flag\n'; exit 1;
}
printf 'ok: v2 launcher forwards the start form flag\n'


before="$(find "$TMP_DIR" -mindepth 1 -maxdepth 1 -printf '%f\n' | sort)"
TMUX="$socket_path,$server_pid,0" DRUDWYN_V2_BIN="$fake_binary" "$ROOT/scripts/v2.sh" status
after="$(find "$TMP_DIR" -mindepth 1 -maxdepth 1 -printf '%f\n' | sort)"
[ "$before" = "$after" ] || {
  printf 'not ok: launcher created unexpected persistent state\n'
  exit 1
}
printf 'ok: v2 launcher creates no project-controlled state\n'

real_binary="$ROOT/target/debug/tmux-drudwyn"
[ -x "$real_binary" ] || {
  printf 'not ok: Rust debug binary is unavailable for lifecycle integration\n'
  exit 1
}
watcher_pid="$(tmux -L "$SOCKET" show-option -gqv @drudwyn_watcher_pid 2>/dev/null || true)"
[ -z "$watcher_pid" ] || kill "$watcher_pid" 2>/dev/null || true
tmux -L "$SOCKET" set-option -gq @drudwyn_watcher_pid ''
tmux -L "$SOCKET" set-option -g @drudwyn-v2 on
ln -s "$(command -v sleep)" "$TMP_DIR/codex"
tmux -L "$SOCKET" new-window -d -t v2 -n agent "$TMP_DIR/codex 30"
agent_pane="$(tmux -L "$SOCKET" list-panes -t v2:agent -F '#{pane_id}' | head -n 1)"
agent_window="$(tmux -L "$SOCKET" display-message -p -t "$agent_pane" '#{window_id}')"
tmux -L "$SOCKET" set-option -wq -t "$agent_window" @drudwyn_message 'sensitive legacy summary'
TMUX="$socket_path,$server_pid,0" DRUDWYN_V2_BIN="$real_binary" "$ROOT/scripts/v2.sh" scan
state="$(tmux -L "$SOCKET" show-option -wqv -t "$agent_window" @drudwyn_state)"
message="$(tmux -L "$SOCKET" show-option -wqv -t "$agent_window" @drudwyn_message)"
agent="$(tmux -L "$SOCKET" show-option -wqv -t "$agent_window" @drudwyn_agent)"
[ "$state" = working ] && [ -z "$message" ] && [ "$agent" = codex ] || {
  printf 'not ok: v2 scan did not classify the process and erase legacy content\n'
  exit 1
}
printf 'ok: v2 scan classifies process identity and erases legacy content\n'

printf '%s' '{"prompt":"private customer material"}' |
  TMUX="$socket_path,$server_pid,0" TMUX_PANE="$agent_pane" DRUDWYN_V2_BIN="$real_binary" \
  "$ROOT/scripts/codex-hook.sh" permissionRequest
state="$(tmux -L "$SOCKET" show-option -wqv -t "$agent_window" @drudwyn_state)"
message="$(tmux -L "$SOCKET" show-option -wqv -t "$agent_window" @drudwyn_message)"
agent="$(tmux -L "$SOCKET" show-option -wqv -t "$agent_window" @drudwyn_agent)"
[ -z "$message" ] && [ "$agent" = codex ] || {
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

tmux -L "$SOCKET" set-option -wq -t "$agent_window" @drudwyn_state needs_input

fleet="$(TMUX="$socket_path,$server_pid,0" DRUDWYN_V2_BIN="$real_binary" \
  "$ROOT/scripts/v2.sh" hud fleet v2 "$agent_window" moon)"
printf '%s' "$fleet" | grep -Fq 'WAITING 1' || {
  printf 'not ok: v2 HUD did not project the live fleet\n'
  exit 1
}
if tmux -L "$SOCKET" show-option -qv -t v2 @drudwyn_sidebar_pane | grep -q .; then
  printf 'not ok: v2 created a sidebar without an explicit opt-in\n'
  exit 1
fi
printf 'ok: v2 HUD projects attention without creating a sidebar\n'

legacy_sidebar="$(tmux -L "$SOCKET" split-window -d -P -F '#{pane_id}' -t v2:)"
tmux -L "$SOCKET" set-option -pq -t "$legacy_sidebar" @drudwyn_sidebar 1
tmux -L "$SOCKET" set-option -q -t v2 @drudwyn_sidebar_pane "$legacy_sidebar"
tmux -L "$SOCKET" run-shell "$ROOT/tmux-drudwyn.tmux"
if tmux -L "$SOCKET" list-panes -a -F '#{pane_id}' | grep -Fxq "$legacy_sidebar"; then
  printf 'not ok: disabling the sidebar left a generated pane behind\n'
  exit 1
fi
printf 'ok: disabling the sidebar removes only its generated pane\n'

sidebar="$(TMUX="$socket_path,$server_pid,0" DRUDWYN_V2_BIN="$real_binary" \
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

tmux -L "$SOCKET" set-option -g @drudwyn-redact-labels on
redacted="$(TMUX="$socket_path,$server_pid,0" DRUDWYN_V2_BIN="$real_binary" \
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
tmux -L "$SOCKET" set-option -g @drudwyn-redact-labels off
printf 'ok: display redaction hides labels without breaking navigation\n'

repo="$TMP_DIR/repo"
worktree_root="$TMP_DIR/worktrees"
git init -q "$repo"
git -C "$repo" config user.name 'Drudwyn Test'
git -C "$repo" config user.email 'drudwyn@example.invalid'
touch "$repo/README.md"
git -C "$repo" add README.md
git -C "$repo" commit -qm initial
git -C "$repo" branch -M main

metadata_failure_bin="$TMP_DIR/metadata-failure-bin"
mkdir "$metadata_failure_bin"
cp "$ROOT/tests/fixtures/tmux_metadata_failure.sh" "$metadata_failure_bin/tmux"
chmod +x "$metadata_failure_bin/tmux"
if TMUX="$socket_path,$server_pid,0" DRUDWYN_TEST_TMUX="$(command -v tmux)" \
  PATH="$metadata_failure_bin:$PATH" "$real_binary" workspace start \
  --repo "$repo" --worktree-root "$worktree_root" metadata/failure sleep 30 \
  >/dev/null 2>&1; then
  printf 'not ok: v2 start unexpectedly succeeded after metadata attachment failed\n'
  exit 1
fi
[ -d "$worktree_root/metadata-failure" ] &&
  git -C "$repo" show-ref --verify --quiet refs/heads/metadata/failure || {
  printf 'not ok: failed v2 start discarded its worktree or branch\n'
  exit 1
}
printf 'ok: failed metadata attachment retains its worktree and branch\n'

TMUX="$socket_path,$server_pid,0" DRUDWYN_V2_BIN="$real_binary" \
  DRUDWYN_WORKTREE_ROOT="$worktree_root" "$ROOT/scripts/worktree-new.sh" \
  --repo "$repo" work/exits-immediately false >/dev/null 2>&1 || true
sleep 0.2
if ! git -C "$repo" show-ref --verify --quiet refs/heads/work/exits-immediately ||
  [ ! -d "$worktree_root/work-exits-immediately" ] ||
  tmux -L "$SOCKET" list-windows -a -F '#{window_name}' | grep -Fxq work-exits-immediately; then
  printf 'not ok: failed v2 start lost retained work or left an unexpected live window\n'
  exit 1
fi
printf 'ok: failed v2 start retains its branch and linked worktree\n'

git -C "$repo" switch -qc ux/pilot
printf 'pilot\n' > "$repo/PILOT.md"
git -C "$repo" add PILOT.md
git -C "$repo" commit -qm pilot
no_change="$(TMUX="$socket_path,$server_pid,0" DRUDWYN_V2_BIN="$real_binary" \
  DRUDWYN_WORKTREE_ROOT="$worktree_root" "$ROOT/scripts/worktree-new.sh" \
  --repo "$repo" --from-current work/no-change "$TMP_DIR/codex" 30)"
[ "$(git -C "$no_change" rev-parse HEAD)" = "$(git -C "$repo" rev-parse ux/pilot)" ] || {
  printf 'not ok: explicit dependent task lost current branch commits\n'; exit 1;
}
TMUX="$socket_path,$server_pid,0" "$real_binary" workspace finish \
  --path "$no_change" --base main --yes >/dev/null
[ ! -e "$no_change" ] || {
  printf 'not ok: finish rejected a clean child of the primary checkout branch\n'
  exit 1
}
printf 'ok: finish accepts work already contained by the primary checkout\n'

created="$(TMUX="$socket_path,$server_pid,0" DRUDWYN_V2_BIN="$real_binary" \
  DRUDWYN_WORKTREE_ROOT="$worktree_root" "$ROOT/scripts/worktree-new.sh" \
  --repo "$repo" work/privacy "$TMP_DIR/codex" 30)"
[ "$created" = "$worktree_root/work-privacy" ] &&
  [ "$(git -C "$created" branch --show-current)" = work/privacy ] || {
  printf 'not ok: v2 start did not create the expected linked worktree\n'
  exit 1
}
created_window="$(tmux -L "$SOCKET" display-message -p -t v2:work-privacy '#{window_id}')"
[ "$(tmux -L "$SOCKET" show-option -wqv -t "$created_window" @drudwyn_message)" = '' ] || {
  printf 'not ok: v2 start created content-bearing tmux state\n'
  exit 1
}
printf 'ok: v2 start creates an isolated workspace without content state\n'

[ "$(git -C "$created" rev-parse HEAD)" = "$(git -C "$repo" rev-parse main)" ] &&
  [ ! -e "$created/PILOT.md" ] || {
  printf 'not ok: independent task inherited unrelated feature commits\n'; exit 1;
}
printf 'ok: independent task starts from main while source is on a feature branch\n'

# A remote ref can be newer than local main; use it without contacting a server.
git -C "$repo" update-ref refs/remotes/origin/main ux/pilot
remote_created="$(TMUX="$socket_path,$server_pid,0" "$real_binary" workspace start \
  --repo "$repo" --worktree-root "$worktree_root" work/remote-base "$TMP_DIR/codex" 30)"
[ "$(git -C "$remote_created" rev-parse HEAD)" = "$(git -C "$repo" rev-parse origin/main)" ] || {
  printf 'not ok: independent task ignored the locally available remote base\n'; exit 1;
}
git -C "$repo" update-ref -d refs/remotes/origin/main
printf 'ok: independent task prefers the remote base over stale local main\n'

# Configuration also governs CLI creation, with an explicit override available.
git -C "$repo" branch trunk ux/pilot
tmux -L "$SOCKET" set-option -g @drudwyn-base-branch trunk
configured="$(TMUX="$socket_path,$server_pid,0" "$real_binary" workspace start \
  --repo "$repo" --worktree-root "$worktree_root" work/configured-base "$TMP_DIR/codex" 30)"
[ "$(git -C "$configured" rev-parse HEAD)" = "$(git -C "$repo" rev-parse trunk)" ] || {
  printf 'not ok: CLI ignored configured base\n'; exit 1;
}
overridden="$(TMUX="$socket_path,$server_pid,0" "$real_binary" workspace start \
  --repo "$repo" --base main --worktree-root "$worktree_root" work/override-base "$TMP_DIR/codex" 30)"
[ "$(git -C "$overridden" rev-parse HEAD)" = "$(git -C "$repo" rev-parse main)" ] || {
  printf 'not ok: explicit base did not override configuration\n'; exit 1;
}
tmux -L "$SOCKET" set-option -gu @drudwyn-base-branch
printf 'ok: CLI respects configured and explicit base branches\n'

if TMUX="$socket_path,$server_pid,0" "$real_binary" workspace start \
  --repo "$repo" --base nonexistent --worktree-root "$worktree_root" \
  work/missing-base "$TMP_DIR/codex" 30 >"$TMP_DIR/missing-base-error" 2>&1; then
  printf 'not ok: missing base silently fell back to HEAD\n'; exit 1;
fi
[ ! -e "$worktree_root/work-missing-base" ] &&
  ! git -C "$repo" show-ref --verify --quiet refs/heads/work/missing-base &&
  grep -q 'starting branch nonexistent is unavailable' "$TMP_DIR/missing-base-error" || {
  printf 'not ok: missing base leaked artifacts or omitted recovery advice\n'; exit 1;
}
printf 'ok: missing base fails before creating branch or worktree\n'

if TMUX="$socket_path,$server_pid,0" "$real_binary" workspace finish \
  --path "$repo" --base main --yes >/dev/null 2>&1; then
  printf 'not ok: v2 finish removed the primary checkout\n'
  exit 1
fi
[ -d "$repo/.git" ] || { printf 'not ok: primary checkout was lost\n'; exit 1; }
printf 'ok: v2 finish refuses the primary checkout\n'

detached="$worktree_root/detached"
git -C "$repo" worktree add -q --detach "$detached"
if TMUX="$socket_path,$server_pid,0" "$real_binary" workspace finish \
  --path "$detached" --base main --yes >/dev/null 2>&1; then
  printf 'not ok: v2 finish removed a detached worktree\n'
  exit 1
fi
[ -d "$detached" ] || { printf 'not ok: detached worktree was lost\n'; exit 1; }
git -C "$repo" worktree remove "$detached"
printf 'ok: v2 finish refuses a detached worktree\n'

linked_created="$(TMUX="$socket_path,$server_pid,0" DRUDWYN_V2_BIN="$real_binary" \
  "$ROOT/scripts/worktree-new.sh" --repo "$created" work/from-linked "$TMP_DIR/codex" 30)"
[ "$linked_created" = "$TMP_DIR/repo-worktrees/work-from-linked" ] || {
  printf 'not ok: start from a linked worktree nested its worktree root: %s\n' "$linked_created"
  exit 1
}
[ "$(git -C "$linked_created" rev-parse HEAD)" = "$(git -C "$repo" rev-parse main)" ] || {
  printf 'not ok: linked-worktree task did not start from the base\n'; exit 1;
}
printf 'ok: start from a linked worktree uses the canonical repository root and base\n'

privacy_task='rotate private customer token 9f47c2'
receiver="$TMP_DIR/task-receiver"
printf '%s\n' '#!/bin/sh' 'IFS= read -r task' 'printf "accepted\n"' 'sleep 2' > "$receiver"
chmod +x "$receiver"
TMUX="$socket_path,$server_pid,0" "$real_binary" workspace start \
  --repo "$repo" --name privacy work/privacy-delivery "$receiver" >/dev/null
privacy_pane="$(tmux -L "$SOCKET" display-message -p -t v2:privacy '#{pane_id}')"
printf '%s' "$privacy_task" | TMUX="$socket_path,$server_pid,0" \
  "$real_binary" workspace deliver-task "$privacy_pane"
sleep 0.2
captured="$(tmux -L "$SOCKET" capture-pane -p -t "$privacy_pane")"
printf '%s' "$captured" | grep -Fq 'accepted' || {
  printf 'not ok: transient task reached the pane but was not submitted\n'
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
  DRUDWYN_V2_BIN="$real_binary" "$ROOT/scripts/worktree-finish.sh") >/dev/null 2>&1; then
  printf 'not ok: v2 finish removed a dirty worktree\n'
  exit 1
fi
[ -d "$created" ] || { printf 'not ok: dirty worktree was lost\n'; exit 1; }
git -C "$created" restore README.md
mkdir "$created/nested"
tmux -L "$SOCKET" respawn-pane -k -t "$created_window" -c "$created/nested" \
  "$TMP_DIR/codex 30"
empty_tree="$(printf '' | git -C "$repo" mktree)"
unrelated_commit="$(printf 'unrelated history\n' | git -C "$repo" commit-tree "$empty_tree")"
git -C "$repo" tag work/privacy "$unrelated_commit"
removed="$(cd "$created" && printf 'y\n' | TMUX="$socket_path,$server_pid,0" \
  DRUDWYN_V2_BIN="$real_binary" "$ROOT/scripts/worktree-finish.sh")"
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
printf 'ok: v2 finish uses branch refs, retains the branch, and closes nested-path windows\n'
