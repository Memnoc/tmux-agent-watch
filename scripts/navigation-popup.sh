#!/usr/bin/env bash
set -eu
PLUGIN_DIR="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"
client="${1:?requesting client required}"
surface="${2:?surface required}"
case "$surface" in
  navigator|cockpit) width=90% height=85% ;;
  sessions) width=96 height=18 ;;
  *) exit 2 ;;
esac
shift 2
# run-shell expands the invoking client; display-popup's command/environment
# arguments do not expand formats on tmux 3.4.
printf -v command '%q ' "$PLUGIN_DIR/scripts/v2.sh" "$surface" "$@"
exec tmux display-popup -c "$client" -EE -w "$width" -h "$height" \
  -e "DRUDWYN_CLIENT=$client" -d '#{pane_current_path}' "$command"
