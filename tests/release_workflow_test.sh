#!/usr/bin/env bash

set -eu

ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
workflow="$ROOT/.github/workflows/release.yml"

for expected in \
  'workflow_dispatch:' \
  'x86_64-unknown-linux-gnu' \
  'aarch64-unknown-linux-gnu' \
  'x86_64-apple-darwin' \
  'aarch64-apple-darwin' \
  "if: startsWith(github.ref, 'refs/tags/v')" \
  'needs: verify' \
  'name: release-bundle'
do
  grep -Fq "$expected" "$workflow" || {
    printf 'not ok: release workflow is missing %s\n' "$expected"
    exit 1
  }
done
[ "$(grep -Fc "if: startsWith(github.ref, 'refs/tags/v')" "$workflow")" -eq 2 ] || {
  printf 'not ok: tag-only guards do not cover version validation and publication\n'
  exit 1
}
printf 'ok: manual preflight builds the full matrix without enabling publication\n'
