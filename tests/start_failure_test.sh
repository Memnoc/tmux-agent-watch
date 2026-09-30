#!/usr/bin/env bash
set -eu
ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
SOCKET="drudwyn-start-failure-$$"
TMP_DIR="$(mktemp -d)"
cleanup() {
  tmux -L "$SOCKET" kill-server 2>/dev/null || true
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT
cargo build --offline --locked --manifest-path "$ROOT/Cargo.toml" >/dev/null
binary="$ROOT/target/debug/tmux-drudwyn"
tmux -L "$SOCKET" -f /dev/null new-session -d -s launch
socket_path="$(tmux -L "$SOCKET" display-message -p '#{socket_path}')"
server_pid="$(tmux -L "$SOCKET" display-message -p '#{pid}')"
export TMUX="$socket_path,$server_pid,0"
repo="$TMP_DIR/repo"
worktrees="$TMP_DIR/worktrees"
git init -q "$repo"
git -C "$repo" config user.name 'Drudwyn Test'
git -C "$repo" config user.email 'drudwyn@example.invalid'
printf 'initial\n' > "$repo/tracked"
git -C "$repo" add tracked
git -C "$repo" commit -qm initial
git -C "$repo" branch -M main
initial="$(git -C "$repo" rev-parse HEAD)"
original_window="$(tmux display-message -p '#{window_id}')"

fail() { printf 'not ok: %s\n' "$*"; exit 1; }
start_fails() {
  branch="$1"; shift
  if "$binary" workspace start --repo "$repo" --worktree-root "$worktrees" --base main "$branch" "$@" >"$TMP_DIR/out" 2>"$TMP_DIR/error"; then
    fail "start reported success for $branch"
  fi
  [ ! -s "$TMP_DIR/out" ] || fail 'failed start printed a success path'
}
assert_retained() {
  branch="$1"
  target="$worktrees/${branch//\//-}"
  [ -d "$target" ] || fail "failed launch removed worktree $target"
  [ "$(git -C "$target" branch --show-current)" = "$branch" ] || fail 'retained branch changed'
  grep -Fq "$target" "$TMP_DIR/error" || fail 'error omitted retained path'
  grep -Fq "$branch" "$TMP_DIR/error" || fail 'error omitted retained branch'
  grep -qi 'launch failed' "$TMP_DIR/error" || fail 'error omitted launch failure'
}

start_fails work/writes sh -c 'printf "partial work\n" > untracked; exit 7'
assert_retained work/writes
[ "$(cat "$target/untracked")" = 'partial work' ] || fail 'worker-written file lost'
[ "$(git -C "$target" status --porcelain)" = '?? untracked' ] || fail 'untracked work lost'
printf 'ok: early failure preserves worker-written untracked files and reports recovery identity\n'

start_fails work/commits sh -c 'printf "committed work\n" > tracked; git add tracked; git commit -qm worker-change; exit 9'
assert_retained work/commits
[ "$(git -C "$target" show HEAD:tracked)" = 'committed work' ] || fail 'worker commit lost'
[ "$(git -C "$target" rev-list --count "$initial..HEAD")" = 1 ] || fail 'worker commit ancestry lost'
[ -z "$(git -C "$target" status --porcelain)" ] || fail 'committed worker should be clean'
printf 'ok: failure preserves newly committed work even when the checkout is clean\n'

for keep_dead in off on; do
  tmux set-option -g remain-on-exit "$keep_dead"
  for status in 0 23; do
    name="work/exit-$keep_dead-$status"
    start_fails "$name" sh -c "exit $status"
    assert_retained "$name"
    if [ "$keep_dead" = on ]; then
      # tmux can retain a dead pane without a native status, even after
      # reaping. Keep that absence explicit instead of inventing the code.
      grep -Eq "exit status ($status|unknown)" "$TMP_DIR/error" || fail 'process exit receipt omitted'
    fi
  done
done
tmux set-option -g remain-on-exit off
printf 'ok: immediate zero and nonzero exits fail launch with and without retained panes\n'

export DRUDWYN_TEST_TMUX="$(command -v tmux)"
injected_bin="$TMP_DIR/injected-bin"
mkdir "$injected_bin"
cp "$ROOT/tests/fixtures/tmux_metadata_failure.sh" "$injected_bin/tmux"
chmod +x "$injected_bin/tmux"
PATH="$injected_bin:$PATH" start_fails work/metadata sh -c 'printf "still working\n" > untracked; exec sleep 60'
assert_retained work/metadata
worker_window="$(tmux display-message -p -t launch:work-metadata '#{window_id}')"
worker_pane="$(tmux display-message -p -t "$worker_window" '#{pane_id}')"
for attempt in {1..100}; do
  [ -f "$target/untracked" ] && break
  sleep 0.02
done
[ "$(cat "$target/untracked")" = 'still working' ] || fail 'metadata failure killed writer'
[ "$(tmux display-message -p -t "$worker_pane" '#{pane_dead}')" = 0 ] || fail 'metadata failure killed live worker'
windows_before="$(tmux list-windows -a -F '#{window_id}')"
start_fails work/metadata sleep 60
grep -Fq 'branch already exists' "$TMP_DIR/error" || fail 'retry did not explain collision'
[ "$(tmux list-windows -a -F '#{window_id}')" = "$windows_before" ] || fail 'retry duplicated surviving worker'
[ "$(cat "$target/untracked")" = 'still working' ] || fail 'retry overwrote retained work'
printf 'ok: metadata failure preserves a live worker; retry cannot duplicate or overwrite it\n'

# A failed tmux response does not prove that no worker was created.
cat > "$injected_bin/tmux" <<'SCRIPT'
#!/usr/bin/env bash
if [ "${1:-}" = new-window ]; then
  "$DRUDWYN_TEST_TMUX" "$@" >/dev/null
  printf 'injected response failure after window creation\n' >&2
  exit 1
fi
exec "$DRUDWYN_TEST_TMUX" "$@"
SCRIPT
PATH="$injected_bin:$PATH" start_fails work/uncertain sh -c 'printf "uncertain work\n" > untracked; exec sleep 60'
assert_retained work/uncertain
for attempt in {1..100}; do
  [ -f "$target/untracked" ] && break
  sleep 0.02
done
[ "$(cat "$target/untracked")" = 'uncertain work' ] || fail 'uncertain tmux failure lost work'
[ "$(tmux display-message -p -t launch:work-uncertain '#{pane_dead}')" = 0 ] || fail 'uncertain tmux failure killed worker'
printf 'ok: failed tmux response retains resources a worker may already be using\n'

start_fails work/missing-agent "$TMP_DIR/agent-does-not-exist"
assert_retained work/missing-agent
printf 'ok: missing worker executable reports failure and retains uncertain allocation\n'

mkdir "$worktrees/work-path-collision"
printf 'existing\n' > "$worktrees/work-path-collision/keep"
start_fails work/path-collision sleep 60
grep -Fq 'worktree path already exists' "$TMP_DIR/error" || fail 'path collision not explained'
[ "$(cat "$worktrees/work-path-collision/keep")" = existing ] || fail 'path collision overwrote files'
! git -C "$repo" show-ref --verify --quiet refs/heads/work/path-collision || fail 'path collision created branch'
[ "$(git -C "$repo" rev-parse HEAD)" = "$initial" ] || fail 'launch failure changed source checkout'
[ "$(tmux display-message -p -t "$original_window" '#{window_id}')" = "$original_window" ] || fail 'launch failure disturbed original window'
printf 'ok: collisions and failed starts preserve unrelated windows and checkouts\n'

# ENOENT from spawning tmux itself proves no window/worker could have started.
missing_tmux_bin="$TMP_DIR/missing-tmux-bin"
mkdir "$missing_tmux_bin"
ln -s "$(command -v git)" "$missing_tmux_bin/git"
PATH="$missing_tmux_bin" start_fails work/unused sleep 60
[ ! -e "$worktrees/work-unused" ] || fail 'provably unused worktree was not cleaned'
! git -C "$repo" show-ref --verify --quiet refs/heads/work/unused || fail 'unused branch was not cleaned'
printf 'ok: clean allocation is removed when the tmux executable never started\n'
