#!/usr/bin/env bash
# Install a complete local runtime snapshot, independent of Git checkout state.
set -eu
ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
destination="${1:-${XDG_DATA_HOME:-$HOME/.local/share}/drudwyn/runtime}"
binary="${DRUDWYN_RUNTIME_BINARY:-$ROOT/target/release/tmux-drudwyn}"
[ -x "$binary" ] || { printf 'Build first: cargo build --release --locked\n' >&2; exit 1; }
mkdir -p "$destination/versions"
destination="$(CDPATH='' cd -- "$destination" && pwd)"
stage="$(mktemp -d "$destination/versions/snapshot.XXXXXXXX")"
# Failed installs cannot change the selected runtime. Old snapshots are retained
# because existing panes/watchers may still be using their entrypoints.
trap 'if [ -n "${stage:-}" ]; then rm -rf "$stage"; fi' EXIT
mkdir -p "$stage/target/release"
install -m 0755 "$binary" "$stage/target/release/tmux-drudwyn"
cp -R "$ROOT/scripts" "$ROOT/assets" "$stage/"
install -m 0755 "$ROOT/tmux-drudwyn.tmux" "$stage/tmux-drudwyn.tmux"
"$stage/target/release/tmux-drudwyn" --version > "$stage/VERSION"
git -C "$ROOT" rev-parse HEAD > "$stage/SOURCE" 2>/dev/null || printf 'local bundle\n' > "$stage/SOURCE"
python3 - "$destination" "$stage" <<'PY'
import os, sys
from pathlib import Path
root, snapshot = map(Path, sys.argv[1:])
link = root / ('.current-' + snapshot.name)
try:
    link.symlink_to(snapshot)
    os.replace(link, root / 'current')
finally:
    link.unlink(missing_ok=True)
PY
stage=''
printf 'Runtime installed: %s/current\n' "$destination"
printf "Tmux loader: run-shell '%s/current/tmux-drudwyn.tmux'\n" "$destination"
printf 'Agent hook: %s/current/scripts/codex-hook.sh\n' "$destination"
