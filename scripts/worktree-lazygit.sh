#!/usr/bin/env bash

set -eu

target="${1:-}"
if [ -n "$target" ]; then
  worktree_path="$(tmux show-option -wqv -t "$target" @agent_watch_worktree 2>/dev/null || true)"
  [ -n "$worktree_path" ] || worktree_path="$(tmux display-message -p -t "$target" '#{pane_current_path}')"
else
  worktree_path="$PWD"
fi
git -C "$worktree_path" rev-parse --is-inside-work-tree >/dev/null 2>&1 || {
  printf 'agent-watch: selected window is not inside a Git worktree\n' >&2
  exit 1
}

lazygit="${AGENT_WATCH_LAZYGIT:-lazygit}"
command -v "$lazygit" >/dev/null 2>&1 || {
  printf 'agent-watch: lazygit is not installed or not on PATH\n' >&2
  exit 1
}

cd "$worktree_path"
exec "$lazygit"
