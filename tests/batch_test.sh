#!/usr/bin/env bash
set -eu
ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
TMP_DIR="$(mktemp -d)"
SOCKET="drudwyn-batch-$$"
cleanup() { tmux -L "$SOCKET" kill-server 2>/dev/null || true; rm -rf "$TMP_DIR"; }
trap cleanup EXIT
cargo build --offline --manifest-path "$ROOT/Cargo.toml" >/dev/null
BIN="$ROOT/target/debug/tmux-drudwyn"
repo="$TMP_DIR/repo"
git init -q -b trunk "$repo"
git -C "$repo" config user.name Test
git -C "$repo" config user.email test@example.invalid
git -C "$repo" commit -qm initial --allow-empty
initial="$(git -C "$repo" rev-parse HEAD)"
tmux -L "$SOCKET" -f /dev/null new-session -d -s batch -c "$repo"
export TMUX="$(tmux -L "$SOCKET" display-message -p '#{socket_path},#{pid},0')"
export TMUX_PANE="$(tmux display-message -p '#{pane_id}')"
unset DRUDWYN_CLIENT || true
session="$(tmux display-message -p '#{session_id}')"
coordinator="$(tmux display-message -p '#{window_id}')"
"$BIN" coordinator set --window "$coordinator" --session "$session"
tmux set-option -g @drudwyn-base-branch trunk
preview="$("$BIN" batch setup --repo "$repo" --session "$session")"
printf '%s' "$preview" | grep -Fq "Source: refs/heads/trunk $initial"
printf '%s' "$preview" | grep -Fq "Destination: trunk $initial"
[ -z "$(tmux show-options -t "$session" | grep '@drudwyn_batch_' || true)" ]
id="$("$BIN" batch setup --repo "$repo" --session "$session" --yes --expect-source "$initial" --expect-destination "$initial" | sed -n 's/^Batch: //p')"
[ -n "$id" ]
"$BIN" batch show "$id" | grep -Fq "Destination: trunk $initial"
# Valid literal branch/path bytes must survive tmux's directory expansion.
"$BIN" workspace start --repo "$repo" --batch "$id" --worktree-root "$TMP_DIR/workers" 'suffix-worker-literal-$value' sleep 90 >/dev/null
literal_window="$(tmux list-windows -t "$session" -F '#{window_id}' | tail -1)"
[ "$(readlink "/proc/$(tmux display-message -p -t "$literal_window" '#{pane_pid}')/cwd")" = "$TMP_DIR/workers/"'suffix-worker-literal-$value' ]
printf 'ok: literal dollar-sign worker path launches in its intended checkout\n'
# Ref movement and navigation cannot make siblings depend on earlier work.
git -C "$repo" commit -qm moved --allow-empty
for branch in one two three; do
  "$BIN" workspace start --repo "$repo" --batch "$id" --worktree-root "$TMP_DIR/workers" "$branch" sleep 90 >/dev/null
  [ "$(git -C "$TMP_DIR/workers/$branch" rev-parse HEAD)" = "$initial" ]
  if [ "$branch" = one ]; then
    git -C "$TMP_DIR/workers/one" commit -qm worker-result --allow-empty
    git -C "$repo" merge -qm integrated one
  fi
done
[ "$(tmux list-windows -t "$session" -F '#{@drudwyn_batch}' | grep -Fc "$id")" -eq 4 ]
worker_pane="$(tmux list-panes -a -F '#{pane_id} #{pane_current_path}' | awk -v p="$TMP_DIR/workers/three" '$2 == p { print $1 }')"
TMUX_PANE="$worker_pane" "$BIN" workspace start --repo "$TMP_DIR/workers/three" --worktree-root "$TMP_DIR/workers" navigated sleep 90 >/dev/null
[ "$(git -C "$TMP_DIR/workers/navigated" rev-parse HEAD)" = "$initial" ]
printf 'ok: preview, live batch and three pinned siblings survive source movement\n'
# A second batch chooses an independent source and destination while planning stays put.
git -C "$repo" switch -qc planning
moved="$(git -C "$repo" rev-parse HEAD)"
printf 'local planning only\n' > "$repo/uncommitted"
args=(batch setup --repo "$repo" --session "$session" --source current --integration assemble --destination-start "$initial" --checkout "$TMP_DIR/assemble")
"$BIN" "${args[@]}" | grep -Fq 'uncommitted files are NOT inherited'
[ ! -e "$TMP_DIR/assemble" ]
second="$("$BIN" "${args[@]}" --yes --expect-source "$moved" --expect-destination "$initial" | sed -n 's/^Batch: //p')"
[ "$(git -C "$repo" branch --show-current)" = planning ]
[ "$(git -C "$TMP_DIR/assemble" rev-parse HEAD)" = "$initial" ]
"$BIN" batch select "$second" --window "$coordinator" >/dev/null
"$BIN" workspace start --repo "$repo" --worktree-root "$TMP_DIR/workers" four sleep 90 >/dev/null
[ "$(git -C "$TMP_DIR/workers/four" rev-parse HEAD)" = "$moved" ]
"$BIN" batch show "$id" | grep -Fq "Source: refs/heads/trunk $initial"
"$BIN" batch show "$second" | grep -Fq "Source: refs/heads/planning $moved"
[ -f "$repo/uncommitted" ]
[ ! -f "$TMP_DIR/workers/four/uncommitted" ]
printf 'ok: independent destination start, live batch selection and dirty-source warning\n'
# Ref movement after review must not silently change what was approved.
if "$BIN" batch setup --repo "$repo" --session "$session" --source current --integration stale --checkout "$TMP_DIR/stale" --yes --expect-source "$initial" --expect-destination "$moved" >"$TMP_DIR/error" 2>&1; then exit 1; fi
grep -Fq 'changed after preview' "$TMP_DIR/error"
[ ! -e "$TMP_DIR/stale" ]
# Existing branches require explicit selection; occupied and missing paths fail before mutation.
if "$BIN" "${args[@]}" >"$TMP_DIR/error" 2>&1; then exit 1; fi
grep -Fq 'already exists' "$TMP_DIR/error"
"$BIN" batch setup --repo "$repo" --session "$session" --integration assemble --reuse-existing | grep -Fq "Checkout: $TMP_DIR/assemble"
for invalid in 'missing-ref' '--help'; do
 if "$BIN" batch setup --repo "$repo" --session "$session" --source="$invalid" >"$TMP_DIR/error" 2>&1; then exit 1; fi
done
if "$BIN" batch setup --repo "$repo" --session "$session" --integration 'bad branch' >"$TMP_DIR/error" 2>&1; then exit 1; fi
if "$BIN" batch setup --repo "$repo" --session "$session" --integration collision --checkout "$repo" >"$TMP_DIR/error" 2>&1; then exit 1; fi
grep -Fq 'path already exists' "$TMP_DIR/error"
"$BIN" batch setup --repo "$repo" --session "$session" --checkout "$TMP_DIR/direct" --yes --expect-source "$moved" --expect-destination "$moved" >/dev/null
[ "$(git -C "$TMP_DIR/direct" branch --show-current)" = trunk ]
[ "$(git -C "$repo" branch --show-current)" = planning ]
printf 'dirty\n' > "$TMP_DIR/direct/dirty"
if "$BIN" batch setup --repo "$repo" --session "$session" >"$TMP_DIR/error" 2>&1; then exit 1; fi
grep -Fq 'Destination checkout is dirty' "$TMP_DIR/error"
git -C "$repo" tag local-source "$initial"
"$BIN" batch setup --repo "$repo" --session "$session" --source local-source --integration assemble --reuse-existing | grep -Fq "Source: local-source $initial"
if "$BIN" batch setup --repo "$repo" --session "$session" --integration assemble --reuse-existing --checkout "$repo" >"$TMP_DIR/error" 2>&1; then exit 1; fi
grep -Fq 'already checked out elsewhere' "$TMP_DIR/error"
printf 'ok: stale previews, local refs, branch/path collisions and dirty destinations guarded\n'
# Destination suffixes and literal refs survive setup, reload, and worker association.
literal_case=0
for suffix in ';' ' ' 'é ' '#{session_name};'; do
  literal_case=$((literal_case + 1))
  case "$suffix" in
    ';') label='literal;' ;;
    ' ') label='literal-space' ;;
    'é ') label='literal-é' ;;
    *) label='literal-$value' ;;
  esac
  checkout="$TMP_DIR/checkout$suffix"
  git -C "$repo" tag "$label" "$initial"
  literal="$("$BIN" batch setup --repo "$repo" --session "$session" --source "$label" --integration "$label" --checkout "$checkout" --yes --expect-source "$initial" --expect-destination "$moved" | sed -n 's/^Batch: //p')"
  [ "$(git -C "$checkout" branch --show-current)" = "$label" ]
  "$BIN" batch show "$literal" > "$TMP_DIR/literal"
  grep -Fxq "Checkout: $checkout" "$TMP_DIR/literal"
  grep -Fxq "Source: $label $initial" "$TMP_DIR/literal"
  grep -Fxq "Destination: $label $moved (at setup)" "$TMP_DIR/literal"
  "$BIN" batch select "$literal" --window "$coordinator" >/dev/null
  worker="suffix-worker-$literal_case"
  "$BIN" workspace start --repo "$repo" --worktree-root "$TMP_DIR/workers" "$worker" sleep 90 >/dev/null
  [ "$(git -C "$TMP_DIR/workers/$worker" rev-parse HEAD)" = "$initial" ]
  worker_window="$(tmux list-windows -t "$session" -F '#{window_id} #{window_name}' | awk -v n="$worker" '$2 == n { print $1 }')"
  [ "$(tmux show-option -wqv -t "$worker_window" @drudwyn_batch)" = "$literal" ]
done
"$BIN" batch select "$second" --window "$coordinator" >/dev/null
printf 'ok: literal refs and checkout suffixes survive live batch reload and association\n'
# Old raw records cannot be trusted after lossy transport; malformed encodings
# must fail before a selected worker can be allocated.
option="@drudwyn_batch_${second#*/}"
record="$(tmux show-option -qv -t "$session" "$option")"
old_record="$(printf '%s\n' "$coordinator" "$repo/.git" planning "$moved" assemble "$initial" "$TMP_DIR/assemble")"
for malformed in "$old_record" 'v2:40' 'v1:40' 'v1:4' 'v1:gg' 'v1:ff' 'v1:40:41:42:43:44:45:' 'v1:40:41:42:43:44:45:0a'; do
  tmux set-option -t "$session" "$option" "$malformed"
  if "$BIN" batch show "$second" >"$TMP_DIR/error" 2>&1; then exit 1; fi
  grep -Fq 'metadata malformed or outdated; reselect source and destination explicitly' "$TMP_DIR/error"
  if "$BIN" workspace start --repo "$repo" --worktree-root "$TMP_DIR/workers" malformed sleep 90 >"$TMP_DIR/error" 2>&1; then exit 1; fi
  grep -Fq 'metadata malformed or outdated' "$TMP_DIR/error"
  [ ! -e "$TMP_DIR/workers/malformed" ]
done
tmux set-option -t "$session" "$option" "$record"
printf 'ok: old and malformed records require explicit reselection before launch\n'
# Loss is unknown, not a guess based on shared repository or last-created batch.
tmux set-option -u -t "$session" "@drudwyn_batch_${second#*/}"
if "$BIN" workspace start --repo "$repo" --worktree-root "$TMP_DIR/workers" lost sleep 90 >"$TMP_DIR/error" 2>&1; then exit 1; fi
grep -Fq 'unknown or lost' "$TMP_DIR/error"
[ ! -e "$TMP_DIR/workers/lost" ]
"$BIN" batch show "$id" | grep -Fq "Source: refs/heads/trunk $initial"
tmux set-option -g @drudwyn-redact-labels on
"$BIN" batch show "$id" > "$TMP_DIR/redacted"
! grep -Fq "$repo" "$TMP_DIR/redacted"
! grep -Fq trunk "$TMP_DIR/redacted"
printf 'ok: missing association is unknown and labels redact\n'
