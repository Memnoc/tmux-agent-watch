#!/usr/bin/env bash

# Clustered status navigation: ordinary workspaces left, current context in the
# centre, and managed agent workspaces right.

set -eu

session="${1:-}"
current="${2:-}"
width="${3:-120}"
icon_mode="$(tmux show-option -gqv @agent-watch-icon-mode 2>/dev/null || true)"
theme="$(tmux show-option -gqv @agent-watch-theme 2>/dev/null || true)"

case "${theme:-moon}" in
  dawn) base='#faf4ed'; surface='#f2e9e1'; highlight='#dfdad9'; text='#575279'; subtle='#797593'; muted='#9893a5'; love='#b4637a'; gold='#ea9d34'; rose='#d7827e'; pine='#286983'; foam='#56949f'; iris='#907aa9' ;;
  rose-pine) base='#191724'; surface='#26233a'; highlight='#403d52'; text='#e0def4'; subtle='#908caa'; muted='#6e6a86'; love='#eb6f92'; gold='#f6c177'; rose='#ebbcba'; pine='#31748f'; foam='#9ccfd8'; iris='#c4a7e7' ;;
  *) base='#232136'; surface='#393552'; highlight='#44415a'; text='#e0def4'; subtle='#908caa'; muted='#6e6a86'; love='#eb6f92'; gold='#f6c177'; rose='#ea9a97'; pine='#3e8fb0'; foam='#9ccfd8'; iris='#c4a7e7' ;;
esac

if [ "$icon_mode" = nerd ]; then
  agent_icon='󰚩'
  branch_icon=''
  more_icon='󰁔'
else
  agent_icon='A'
  branch_icon='git:'
  more_icon='>>'
fi

age_label() {
  local since="$1" elapsed
  case "$since" in ''|*[!0-9]*) printf 'now'; return ;; esac
  elapsed=$(( $(date +%s) - since ))
  if [ "$elapsed" -lt 60 ]; then
    printf '%ss' "$elapsed"
  elif [ "$elapsed" -lt 3600 ]; then
    printf '%sm' "$((elapsed / 60))"
  elif [ "$elapsed" -lt 86400 ]; then
    printf '%sh' "$((elapsed / 3600))"
  else
    printf '%sd' "$((elapsed / 86400))"
  fi
}

git_context() {
  local repo="$1" branch="$2" state="$3" available="$4" stats added deleted status files untracked state_context='' branch_label
  [ -n "$repo" ] && git -C "$repo" rev-parse --is-inside-work-tree >/dev/null 2>&1 || return
  [ -n "$branch" ] || branch="$(git -C "$repo" branch --show-current 2>/dev/null || true)"
  stats="$(git -C "$repo" diff --numstat HEAD -- 2>/dev/null |
    awk '$1 ~ /^[0-9]+$/ { add += $1 } $2 ~ /^[0-9]+$/ { del += $2 } END { print add+0, del+0 }')"
  read -r added deleted <<EOF
$stats
EOF
  status="$(git -C "$repo" status --porcelain 2>/dev/null || true)"
  files="$(printf '%s\n' "$status" | awk 'NF { count++ } END { print count+0 }')"
  untracked="$(printf '%s\n' "$status" | awk 'substr($0,1,2) == "??" { count++ } END { print count+0 }')"
  case "$state" in
    needs_input) state_context="#[fg=${gold}]WAITING #[fg=${muted}]· " ;;
    done) state_context="#[fg=${pine}]REVIEW #[fg=${muted}]· " ;;
    failed) state_context="#[fg=${love}]FAILED #[fg=${muted}]· " ;;
  esac
  branch_label="${branch:-detached}"
  if [ "$available" -lt 105 ]; then
    branch_label="$(printf '%s' "$branch_label" | cut -c1-8)"
  elif [ "$available" -lt 120 ]; then
    branch_label="$(printf '%s' "$branch_label" | cut -c1-14)"
  fi
  if [ "$files" -eq 0 ]; then
    printf '%s#[fg=%s]%s #[fg=%s]%s  #[fg=%s]✓ clean#[default]' \
      "$state_context" "$iris" "$branch_icon" "$text" "$branch_label" "$pine"
    return
  fi
  if [ "$available" -lt 105 ]; then
    printf '%s#[fg=%s]%s  #[fg=%s]+%s #[fg=%s]−%s#[default]' \
      "$state_context" "$text" "$branch_label" "$pine" "$added" "$love" "$deleted"
    return
  fi
  printf '%s#[fg=%s]%s #[fg=%s]%s  #[fg=%s]+%s #[fg=%s]−%s  #[fg=%s]%s files' \
    "$state_context" "$iris" "$branch_icon" "$text" "$branch_label" \
    "$pine" "$added" "$love" "$deleted" "$subtle" "$files"
  if [ "$untracked" -gt 0 ] && [ "$available" -ge 120 ]; then
    printf '  #[fg=%s]?%s' "$gold" "$untracked"
  fi
  printf '#[default]'
}

compact_bar() {
  local repo="$1" branch="$2" state="$3" name="$4" available="$5"
  local project project_limit branch_limit status stats added deleted agent_label agent_color

  project="${repo##*/}"
  [ -n "$project" ] || project="$name"
  if [ "$available" -lt 50 ]; then
    project_limit=6; branch_limit=7
  elif [ "$available" -lt 65 ]; then
    project_limit=10; branch_limit=10
  else
    project_limit=14; branch_limit=14
  fi
  project="$(printf '%s' "$project" | cut -c1-"$project_limit")"

  printf '#[align=left,fg=%s,bold]%s' "$text" "$project"
  if [ -n "$repo" ] && git -C "$repo" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    [ -n "$branch" ] || branch="$(git -C "$repo" branch --show-current 2>/dev/null || true)"
    branch="$(printf '%s' "${branch:-detached}" | cut -c1-"$branch_limit")"
    status="$(git -C "$repo" status --porcelain 2>/dev/null || true)"
    printf ' #[fg=%s]· %s #[fg=%s]%s' "$muted" "$branch_icon" "$text" "$branch"
    if [ -n "$status" ]; then
      if [ "$available" -lt 50 ]; then
        printf ' #[fg=%s]●' "$gold"
      else
        stats="$(git -C "$repo" diff --numstat HEAD -- 2>/dev/null |
          awk '$1 ~ /^[0-9]+$/ { add += $1 } $2 ~ /^[0-9]+$/ { del += $2 } END { print add+0, del+0 }')"
        read -r added deleted <<EOF
$stats
EOF
        printf ' #[fg=%s]+%s #[fg=%s]−%s' "$pine" "$added" "$love" "$deleted"
      fi
    else
      printf ' #[fg=%s]✓' "$pine"
    fi
  fi

  case "$state" in
    working) agent_label='WORK'; agent_color="$foam" ;;
    needs_input) agent_label='WAIT'; agent_color="$gold" ;;
    done) agent_label='REVIEW'; agent_color="$pine" ;;
    failed) agent_label='FAIL'; agent_color="$love" ;;
    *) agent_label=''; agent_color="$iris" ;;
  esac
  if [ -n "$agent_label" ]; then
    printf ' #[fg=%s]· #[fg=%s]%s %s' "$muted" "$agent_color" "$agent_icon" "$agent_label"
  fi
  printf '  #[fg=%s]%s#[default]' "$iris" "$more_icon"
}

rows="$(tmux list-windows -t "$session" \
  -F '#{window_index}|#{window_id}|#{window_name}|#{window_active}|#{@agent_watch_state}|#{@agent_watch_branch}|#{@agent_watch_repo}|#{@agent_watch_git_status}|#{pane_current_path}|#{@agent_watch_since}|#{@agent_watch_context_repo}' \
  2>/dev/null || true)"
current_is_agent="$(printf '%s\n' "$rows" | awk -F'|' -v id="$current" '$2 == id && $5 != "" { print 1; exit }')"

# Keep each cluster bounded so context remains readable; terminal-width tiers
# avoid compressing labels into noise on smaller clients.
if [ "$width" -lt 105 ]; then
  left_limit=0; right_limit=1; inactive_name_limit=0; agent_name_limit=7
elif [ "$width" -lt 120 ]; then
  left_limit=2; right_limit=2; inactive_name_limit=4; agent_name_limit=8
elif [ "$width" -lt 140 ]; then
  left_limit=3; right_limit=3; inactive_name_limit=7; agent_name_limit=10
else
  left_limit=3; right_limit=4; inactive_name_limit=7; agent_name_limit=12
fi

left=''
right=''
left_count=0
right_count=0
left_hidden=0
right_hidden=0
context=''
current_name=''
current_state=''
current_branch=''
current_repo=''

while IFS='|' read -r index window_id name _active state branch repo git_status path since context_repo; do
  [ -n "$window_id" ] || continue
  short_name="$(printf '%s' "$name" | cut -c1-"$agent_name_limit")"
  agent=0
  [ -n "$state" ] && agent=1

  if [ "$window_id" = "$current" ]; then
    repo="${context_repo:-${repo:-$path}}"
    current_name="$name"
    current_state="$state"
    current_branch="$branch"
    current_repo="$repo"
    if [ -n "$repo" ]; then
      context="$(git_context "$repo" "$branch" "$state" "$width")"
    elif [ -n "$state" ]; then
      case "$state" in
        working) state_label='WORKING'; state_detail='active'; context_color="$foam" ;;
        needs_input) state_label='WAITING'; state_detail='needs you'; context_color="$gold" ;;
        done) state_label='REVIEW'; state_detail='ready'; context_color="$pine" ;;
        failed) state_label='FAILED'; state_detail='stopped'; context_color="$love" ;;
        *) state_label='AGENT'; state_detail='active'; context_color="$iris" ;;
      esac
      age="$(age_label "$since")"
      branch_context=''
      if [ -n "$branch" ]; then
        branch_context="  #[fg=${iris}]${branch_icon} #[fg=${subtle}]${branch}"
      fi
      context="#[fg=${context_color}]${agent_icon} ${state_label} #[fg=${muted}]· ${state_detail} ${age}${branch_context}#[default]"
    elif [ -n "$branch" ]; then
      repo_name="${repo##*/}"
      dirty=''
      if [ "$git_status" = dirty ]; then
        dirty=" #[fg=${gold}]●"
      fi
      if [ "$repo_name" = "$name" ]; then
        context="#[fg=${iris}]${branch_icon} #[fg=${text}]${branch}${dirty}#[default]"
      else
        context="#[fg=${subtle}]${repo_name}  #[fg=${iris}]${branch_icon} #[fg=${text}]${branch}${dirty}#[default]"
      fi
    else
      context=''
    fi
  fi

  if [ "$agent" = 1 ]; then
    right_count=$((right_count + 1))
    show_agent=0
    if [ "$width" -lt 105 ] && [ "$current_is_agent" = 1 ]; then
      [ "$window_id" = "$current" ] && show_agent=1
    elif [ "$right_count" -le "$right_limit" ] || [ "$window_id" = "$current" ]; then
      show_agent=1
    fi
    if [ "$show_agent" = 1 ]; then
      case "$state" in
        failed) color="$love" ;;
        needs_input) color="$gold" ;;
        done) color="$pine" ;;
        *) color="$foam" ;;
      esac
      if [ "$window_id" = "$current" ]; then
        item="#[range=window|${index}]#[bg=default,fg=${iris},bold]▶ #[fg=${color}]${agent_icon} ${index} ${short_name}#[norange]"
      else
        item="#[range=window|${index}]#[fg=${color}]${agent_icon} #[fg=${subtle}]${index} ${short_name}#[norange]"
      fi
      right="${right}  ${item}"
    else
      right_hidden=$((right_hidden + 1))
    fi
  else
    left_count=$((left_count + 1))
    if [ "$left_count" -le "$left_limit" ] || [ "$window_id" = "$current" ]; then
      if [ "$window_id" = "$current" ]; then
        if [ "$icon_mode" = nerd ]; then
          item="#[range=window|${index}]#[fg=${rose}]#[bg=${surface},fg=${text},bold] ${index} ${short_name} #[bg=default,fg=${rose}]#[norange]"
        else
          item="#[range=window|${index}]#[fg=${rose},bold][ #[fg=${text}]${index} ${short_name} #[fg=${rose}] ]#[norange]"
        fi
      else
        if [ "$inactive_name_limit" -eq 0 ]; then
          item="#[range=window|${index}]#[fg=${muted}]${index}#[norange]"
        else
          compact_name="$(printf '%s' "$name" | cut -c1-"$inactive_name_limit")"
          item="#[range=window|${index}]#[fg=${muted}]${index} ${compact_name}#[norange]"
        fi
      fi
      left="${left}  ${item}"
    else
      left_hidden=$((left_hidden + 1))
    fi
  fi
done <<EOF
$rows
EOF

if [ "$width" -lt 80 ]; then
  compact_bar "$current_repo" "$current_branch" "$current_state" "$current_name" "$width"
  exit 0
fi

if [ "$left_hidden" -gt 0 ]; then
  left="${left}  #[fg=${iris}]${more_icon}#[default]"
fi
if [ "$right_hidden" -gt 0 ]; then
  right="${right}  #[fg=${gold}]${more_icon}#[default]"
fi
if [ -n "$right" ]; then
  right="#[fg=${iris}]│#[default]${right}  "
fi

printf '#[align=left]%s#[align=centre]%s#[align=right]%s#[default]' "$left" "$context" "$right"
