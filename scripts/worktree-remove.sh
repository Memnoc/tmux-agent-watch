#!/usr/bin/env bash

set -eu

usage() {
  printf 'usage: %s BRANCH\n' "$0" >&2
  exit 2
}

branch="${1:-}"
[ -n "$branch" ] && [ "$#" -eq 1 ] || usage

repo="$(git rev-parse --show-toplevel 2>/dev/null)" || {
  printf 'agent-watch: current directory is not inside a Git repository\n' >&2
  exit 1
}
repo_name="${repo##*/}"
worktree_root="${AGENT_WATCH_WORKTREE_ROOT:-${repo%/*}/${repo_name}-worktrees}"
slug="${branch//\//-}"
worktree="$worktree_root/$slug"

[ -d "$worktree" ] || {
  printf 'agent-watch: worktree does not exist: %s\n' "$worktree" >&2
  exit 1
}
actual_branch="$(git -C "$worktree" branch --show-current 2>/dev/null || true)"
[ "$actual_branch" = "$branch" ] || {
  printf 'agent-watch: %s contains branch %s, not %s\n' \
    "$worktree" "${actual_branch:-<detached>}" "$branch" >&2
  exit 1
}

window_ids="$(tmux list-panes -a -F '#{window_id}|#{pane_current_path}' 2>/dev/null |
  awk -F '|' -v path="$worktree" '$2 == path && !seen[$1]++ { print $1 }' || true)"

if ! git -C "$repo" worktree remove "$worktree"; then
  printf 'agent-watch: worktree was not removed; commit or discard its changes first\n' >&2
  exit 1
fi

while IFS= read -r window_id; do
  [ -n "$window_id" ] && tmux kill-window -t "$window_id" 2>/dev/null || true
done <<< "$window_ids"

printf '%s\n' "$worktree"
