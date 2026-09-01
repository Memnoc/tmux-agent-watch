#!/usr/bin/env bash

set -eu

worktree="$(git rev-parse --show-toplevel 2>/dev/null)" || {
  printf 'agent-watch: current directory is not inside a Git worktree\n' >&2
  exit 1
}
git_dir="$(git -C "$worktree" rev-parse --absolute-git-dir)"
common_dir="$(git -C "$worktree" rev-parse --path-format=absolute --git-common-dir)"
[ "$git_dir" != "$common_dir" ] || {
  printf 'agent-watch: the primary checkout cannot be finished as a linked worktree\n' >&2
  exit 1
}

branch="$(git -C "$worktree" branch --show-current)"
[ -n "$branch" ] || {
  printf 'agent-watch: detached worktrees must be handled manually\n' >&2
  exit 1
}
[ -z "$(git -C "$worktree" status --porcelain)" ] || {
  printf 'agent-watch: worktree is dirty; review, commit, or discard its changes first\n' >&2
  exit 1
}

main_worktree="$(git -C "$worktree" worktree list --porcelain |
  awk '/^worktree / { sub(/^worktree /, ""); print; exit }')"
base_branch="$(tmux show-option -gqv @agent-watch-base-branch 2>/dev/null || true)"
base_branch="${base_branch:-main}"
git -C "$main_worktree" show-ref --verify --quiet "refs/heads/$base_branch" || {
  printf 'agent-watch: base branch %s does not exist; set @agent-watch-base-branch\n' "$base_branch" >&2
  exit 1
}
git -C "$main_worktree" merge-base --is-ancestor "$branch" "$base_branch" || {
  printf 'agent-watch: %s is not merged into %s; review and merge it first\n' "$branch" "$base_branch" >&2
  exit 1
}

printf 'Remove linked worktree for %s? The branch will be retained. [y/N] ' "$branch" >&2
read -r answer
case "$answer" in y|Y|yes|YES) ;; *) printf 'Cancelled.\n' >&2; exit 1 ;; esac

window_ids="$(tmux list-panes -a -F '#{window_id}|#{pane_current_path}' 2>/dev/null |
  awk -F '|' -v path="$worktree" '$2 == path && !seen[$1]++ { print $1 }' || true)"
cd "$main_worktree"
git worktree remove "$worktree"

while IFS= read -r window_id; do
  [ -n "$window_id" ] && tmux kill-window -t "$window_id" 2>/dev/null || true
done <<< "$window_ids"

printf '%s\n' "$worktree"
