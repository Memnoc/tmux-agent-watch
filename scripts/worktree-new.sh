#!/usr/bin/env bash

set -eu

usage() {
  printf 'usage: %s [--repo PATH] BRANCH [AGENT [ARG ...]]\n' "$0" >&2
  exit 2
}

repo_hint=''
if [ "${1:-}" = --repo ]; then
  [ -n "${2:-}" ] || usage
  repo_hint="$2"
  shift 2
fi

branch="${1:-}"
[ -n "$branch" ] || usage
shift

git check-ref-format --branch "$branch" >/dev/null 2>&1 || {
  printf 'agent-watch: invalid branch name: %s\n' "$branch" >&2
  exit 2
}

repo="$(git -C "${repo_hint:-.}" rev-parse --show-toplevel 2>/dev/null)" || {
  printf 'agent-watch: current directory is not inside a Git repository\n' >&2
  exit 1
}
repo_name="${repo##*/}"
worktree_root="${AGENT_WATCH_WORKTREE_ROOT:-${repo%/*}/${repo_name}-worktrees}"
slug="${branch//\//-}"
worktree="$worktree_root/$slug"

[ ! -e "$worktree" ] || {
  printf 'agent-watch: worktree path already exists: %s\n' "$worktree" >&2
  exit 1
}
git -C "$repo" show-ref --verify --quiet "refs/heads/$branch" && {
  printf 'agent-watch: branch already exists: %s\n' "$branch" >&2
  exit 1
}

mkdir -p "$worktree_root"
git -C "$repo" worktree add -q -b "$branch" "$worktree"

if [ "$#" -eq 0 ]; then
  agent="${AGENT_WATCH_AGENT:-$(tmux show-option -gqv @agent-watch-agent 2>/dev/null || true)}"
  set -- "${agent:-codex}"
fi

command_string=''
for argument in "$@"; do
  printf -v quoted '%q' "$argument"
  command_string="${command_string:+$command_string }$quoted"
done

if ! window_id="$(tmux new-window -d -P -F '#{window_id}' -n "$slug" -c "$worktree" "$command_string")"; then
  git -C "$repo" worktree remove "$worktree" 2>/dev/null || true
  git -C "$repo" branch -D "$branch" >/dev/null 2>&1 || true
  exit 1
fi
tmux set-option -wq -t "$window_id" @agent_watch_branch "$branch"
tmux set-option -wq -t "$window_id" @agent_watch_worktree "$worktree"
tmux set-option -wq -t "$window_id" @agent_watch_repo "$worktree"
tmux set-option -wq -t "$window_id" @agent_watch_git_status clean
tmux set-option -wq -t "$window_id" @agent_watch_git_checked "$(date +%s)"

printf '%s\n' "$worktree"
