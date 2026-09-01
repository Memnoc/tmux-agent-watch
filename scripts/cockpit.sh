#!/usr/bin/env bash

set -u

PLUGIN_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)"

repo="$(git rev-parse --show-toplevel 2>/dev/null || true)"
repo_name="${repo##*/}"
[ -n "$repo" ] || repo_name='not a Git repository'

color() { printf '\033[38;2;%sm' "$1"; }
reset=$'\033[0m'
bold=$'\033[1m'

render() {
  local rows working waiting review dirty worktrees
  rows="$(tmux list-windows -a -F '#{@agent_watch_state}|#{@agent_watch_worktree}|#{@agent_watch_git_status}' 2>/dev/null || true)"
  working="$(printf '%s\n' "$rows" | awk -F '|' '$1=="working" {n++} END {print n+0}')"
  waiting="$(printf '%s\n' "$rows" | awk -F '|' '$1=="needs_input" || $1=="failed" {n++} END {print n+0}')"
  review="$(printf '%s\n' "$rows" | awk -F '|' '$1=="done" {n++} END {print n+0}')"
  worktrees="$(printf '%s\n' "$rows" | awk -F '|' '$2!="" {n++} END {print n+0}')"
  dirty="$(printf '%s\n' "$rows" | awk -F '|' '$2!="" && $3=="dirty" {n++} END {print n+0}')"

  printf '\033[H\033[J'
  printf '  %b%sWORKSPACE COMMAND CENTER%b\n' "$(color '196;167;231')" "$bold" "$reset"
  printf '  %b%s%b\n\n' "$(color '144;140;170')" "tmux-agent-watch · Git worktrees without the ceremony" "$reset"
  printf '  %b› What do you want to do?%b\n\n' "$bold" "$reset"
  printf '  %b[1]%b  %bStart a quick win%b\n' "$(color '246;193;119')" "$reset" "$bold" "$reset"
  printf '       Create a branch, linked worktree, and agent window\n\n'
  printf '  %b[2]%b  %bReview ready work%b\n' "$(color '166;218;149')" "$reset" "$bold" "$reset"
  printf '       Open a waiting, failed, or completed workspace\n\n'
  printf '  %b[3]%b  %bJump to a workspace%b\n' "$(color '156;207;216')" "$reset" "$bold" "$reset"
  printf '       Find any agent by project, branch, or state\n\n'
  printf '  %b[4]%b  %bFinish merged work%b\n' "$(color '196;167;231')" "$reset" "$bold" "$reset"
  printf '       Remove a clean merged worktree safely\n\n'
  printf '  %bCurrent repo:%b %s' "$(color '144;140;170')" "$reset" "$repo_name"
  printf '   %s worktrees · %s dirty\n' "$worktrees" "$dirty"
  printf '  %bFleet:%b %s working · %s waiting · %s review\n\n' "$(color '144;140;170')" "$reset" "$working" "$waiting" "$review"
  printf '  %b1-4 choose · q/Esc close%b' "$(color '144;140;170')" "$reset"
}

pause_with_error() {
  printf '\n  %b%s%b\n' "$(color '235;111;146')" "$1" "$reset"
  printf '  %bPress any key to return%b' "$(color '144;140;170')" "$reset"
  IFS= read -rsn1 _
}

start_workspace() {
  local task slug branch agent_key agent output target
  [ -n "$repo" ] || { pause_with_error 'Start requires a Git repository.'; return; }
  printf '\033[H\033[J'
  printf '  %b%sSTART A QUICK WIN%b\n\n' "$(color '246;193;119')" "$bold" "$reset"
  printf '  Current repo  %s\n\n' "$repo_name"
  printf '  Describe the task in a few words:\n  %b›%b ' "$(color '196;167;231')" "$reset"
  IFS= read -r task
  slug="$(printf '%s' "$task" | tr '[:upper:]' '[:lower:]' |
    sed -E 's/[^a-z0-9]+/-/g; s/^-+//; s/-+$//')"
  [ -n "$slug" ] || { pause_with_error 'Task name must contain a letter or number.'; return; }
  branch="quick-win/$slug"

  printf '\033[H\033[J'
  printf '  %b%sCHOOSE AN AGENT%b\n\n' "$(color '196;167;231')" "$bold" "$reset"
  printf '  Task    %s\n' "$task"
  printf '  Branch  %b%s%b\n\n' "$(color '196;167;231')" "$branch" "$reset"
  printf '  %b[1]%b Codex      %b[2]%b Claude      %b[3]%b OpenCode\n\n' \
    "$(color '246;193;119')" "$reset" "$(color '246;193;119')" "$reset" \
    "$(color '246;193;119')" "$reset"
  while IFS= read -rsn1 agent_key; do
    case "$agent_key" in 1) agent=codex; break ;; 2) agent=claude; break ;; 3) agent=opencode; break ;; $'\033') return ;; esac
  done

  printf '\033[H\033[J'
  printf '  %b%sREADY TO CREATE%b\n\n' "$(color '166;218;149')" "$bold" "$reset"
  printf '  Repository  %s\n' "$repo"
  printf '  Branch      %b%s%b\n' "$(color '196;167;231')" "$branch" "$reset"
  printf '  Agent       %s\n\n' "$agent"
  printf '  %bEnter create · Esc cancel%b' "$(color '144;140;170')" "$reset"
  IFS= read -rsn1 agent_key
  [ "$agent_key" != $'\033' ] || return

  printf '\n\n  Creating workspace…\n'
  if ! output="$($PLUGIN_DIR/worktree-new.sh --repo "$repo" "$branch" "$agent" 2>&1)"; then
    pause_with_error "$output"
    return
  fi
  target="$(tmux list-windows -a -F '#{window_id}|#{@agent_watch_worktree}' 2>/dev/null |
    awk -F '|' -v path="$output" '$2 == path { print $1; exit }')"
  [ -n "$target" ] && tmux select-window -t "$target" 2>/dev/null || true
  exit 0
}

review_workspace() {
  local rows count line choice window_id branch state message label
  rows="$(tmux list-windows -a -F '#{window_id}|#{@agent_watch_branch}|#{@agent_watch_state}|#{@agent_watch_message}|#{@agent_watch_attention_since}' 2>/dev/null |
    awk -F '|' '$3=="done" || $3=="failed" || $3=="needs_input" { print }' |
    sort -t '|' -k5,5n)"
  [ -n "$rows" ] || { pause_with_error 'No workspaces currently need review or input.'; return; }

  printf '\033[H\033[J'
  printf '  %b%sREVIEW READY WORK%b\n\n' "$(color '166;218;149')" "$bold" "$reset"
  count=0
  while IFS='|' read -r window_id branch state message _; do
    count=$((count + 1)); [ "$count" -le 9 ] || break
    case "$state" in done) label=REVIEW ;; failed) label=FAILED ;; needs_input) label=WAITING ;; esac
    printf '  %b[%s]%b %b%s%b  %s\n' "$(color '166;218;149')" "$count" "$reset" "$bold" "${branch:-workspace}" "$reset" "$label"
    printf '      %s\n\n' "${message:-No summary available}"
  done <<< "$rows"
  printf '  %b1-%s jump to workspace · Esc back%b' "$(color '144;140;170')" "$count" "$reset"
  while IFS= read -rsn1 choice; do
    [ "$choice" != $'\033' ] || return
    case "$choice" in [1-9])
      line="$(printf '%s\n' "$rows" | sed -n "${choice}p")"
      [ -n "$line" ] || continue
      IFS='|' read -r window_id _ <<< "$line"
      session="$(tmux display-message -p -t "$window_id" '#{session_name}')"
      tmux switch-client -t "$session" 2>/dev/null || true
      tmux select-window -t "$window_id"
      exit 0
      ;;
    esac
  done
}

jump_workspace() {
  local rows count line choice window_id session name state branch label
  rows="$(tmux list-windows -a -F '#{window_id}|#{session_name}|#{window_name}|#{@agent_watch_state}|#{@agent_watch_branch}' 2>/dev/null |
    awk -F '|' '$4!="" { print }')"
  [ -n "$rows" ] || { pause_with_error 'No live agent workspaces were found.'; return; }

  printf '\033[H\033[J'
  printf '  %b%sJUMP TO A WORKSPACE%b\n\n' "$(color '156;207;216')" "$bold" "$reset"
  count=0
  while IFS='|' read -r window_id session name state branch; do
    count=$((count + 1)); [ "$count" -le 9 ] || break
    case "$state" in done) label=REVIEW ;; needs_input) label=WAITING ;; *) label="${state^^}" ;; esac
    printf '  %b[%s]%b %b%-24s%b %s\n' "$(color '156;207;216')" "$count" "$reset" "$bold" "${branch:-$name}" "$reset" "$label"
    printf '      %s:%s\n\n' "$session" "$name"
  done <<< "$rows"
  printf '  %b1-%s jump · Esc back%b' "$(color '144;140;170')" "$count" "$reset"
  while IFS= read -rsn1 choice; do
    [ "$choice" != $'\033' ] || return
    case "$choice" in [1-9])
      line="$(printf '%s\n' "$rows" | sed -n "${choice}p")"
      [ -n "$line" ] || continue
      IFS='|' read -r window_id session _ <<< "$line"
      tmux switch-client -t "$session" 2>/dev/null || true
      tmux select-window -t "$window_id"
      exit 0
      ;;
    esac
  done
}

finish_workspace() {
  local candidates rows window_id branch worktree status main_worktree base_branch
  local count choice line
  rows="$(tmux list-windows -a -F '#{window_id}|#{@agent_watch_branch}|#{@agent_watch_worktree}|#{@agent_watch_git_status}' 2>/dev/null || true)"
  candidates=''
  while IFS='|' read -r window_id branch worktree status; do
    [ -n "$worktree" ] && [ "$status" = clean ] && [ -n "$branch" ] || continue
    main_worktree="$(git -C "$worktree" worktree list --porcelain 2>/dev/null |
      awk '/^worktree / { sub(/^worktree /, ""); print; exit }')"
    [ -n "$main_worktree" ] || continue
    base_branch="$(tmux show-option -gqv @agent-watch-base-branch 2>/dev/null || true)"
    base_branch="${base_branch:-main}"
    git -C "$main_worktree" merge-base --is-ancestor "$branch" "$base_branch" 2>/dev/null || continue
    candidates="${candidates}${candidates:+$'\n'}$window_id|$branch|$worktree|$base_branch"
  done <<< "$rows"
  [ -n "$candidates" ] || {
    pause_with_error 'No clean merged worktrees are ready to finish.'
    return
  }

  printf '\033[H\033[J'
  printf '  %b%sFINISH MERGED WORK%b\n\n' "$(color '196;167;231')" "$bold" "$reset"
  count=0
  while IFS='|' read -r window_id branch worktree base_branch; do
    count=$((count + 1)); [ "$count" -le 9 ] || break
    printf '  %b[%s]%b %b%s%b\n' "$(color '196;167;231')" "$count" "$reset" "$bold" "$branch" "$reset"
    printf '      CLEAN · merged into %s\n' "$base_branch"
    printf '      %s\n\n' "$worktree"
  done <<< "$candidates"
  printf '  %b1-%s finish · Esc back%b' "$(color '144;140;170')" "$count" "$reset"
  while IFS= read -rsn1 choice; do
    [ "$choice" != $'\033' ] || return
    case "$choice" in [1-9])
      line="$(printf '%s\n' "$candidates" | sed -n "${choice}p")"
      [ -n "$line" ] || continue
      IFS='|' read -r _ _ worktree _ <<< "$line"
      cd "$worktree"
      exec "$PLUGIN_DIR/worktree-finish.sh"
      ;;
    esac
  done
}

render
while IFS= read -rsn1 key; do
  case "$key" in
    q|$'\033') exit 0 ;;
    1) start_workspace; render ;;
    2) review_workspace; render ;;
    3) jump_workspace; render ;;
    4) finish_workspace; render ;;
  esac
done
