#!/usr/bin/env bash

set -eu

ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"

for dependency in reqwest hyper ureq curl sentry tracing-subscriber rusqlite sqlx; do
  if awk '/^\[dependencies\]/{inside=1; next} /^\[/{inside=0} inside {print}' "$ROOT/Cargo.toml" |
    grep -Eq "^[[:space:]]*$dependency[[:space:]]*="; then
    printf 'not ok: privacy boundary includes network, telemetry, or storage dependency %s\n' "$dependency"
    exit 1
  fi
done
printf 'ok: release dependencies contain no network, telemetry, or database clients\n'

if rg -n 'TcpStream|TcpListener|UdpSocket|reqwest|hyper::|sentry|analytics|telemetry|rusqlite|sqlx' \
  "$ROOT/src" >/dev/null; then
  printf 'not ok: Rust v2 contains a forbidden remote, telemetry, or database path\n'
  exit 1
fi
printf 'ok: Rust v2 contains no remote, telemetry, or database path\n'

[ -f "$ROOT/docs/privacy.md" ] || {
  printf 'not ok: local data-flow manifest is missing\n'
  exit 1
}
printf 'ok: release includes a local data-flow manifest\n'

grep -Fq 'The default Rust implementation (`@agent-watch-v2 on`)' "$ROOT/docs/privacy.md" || {
  printf 'not ok: privacy manifest does not scope its content-blind promise to Rust v2\n'
  exit 1
}
grep -Fq 'tmux capture-pane' "$ROOT/docs/privacy.md" || {
  printf 'not ok: privacy manifest does not disclose legacy scrollback processing\n'
  exit 1
}
tr '\n' ' ' < "$ROOT/README.md" |
  grep -Fq 'The default Rust implementation never reads' || {
  printf 'not ok: README privacy claim is not scoped to the default Rust implementation\n'
  exit 1
}
printf 'ok: public privacy claims distinguish Rust v2 from the legacy fallback\n'
