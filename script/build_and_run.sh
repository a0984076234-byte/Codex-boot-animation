#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export DRAGON_BUILD_DIR="${DRAGON_BUILD_DIR:-/private/tmp/dragon-codex-boot-local}"
mode="${1:-run}"
if [[ $# -gt 0 ]]; then shift; fi

# Stop only prior preview instances; leave the installed launch monitor running.
while IFS= read -r preview_pid; do
  [[ -z "$preview_pid" ]] || kill "$preview_pid"
done < <(pgrep -f "^$DRAGON_BUILD_DIR/DragonCodexBoot.app/Contents/MacOS/DragonCodexBootMac( |$)" || true)
bash "$repo_root/scripts/build-macos.sh"
app_bundle="$DRAGON_BUILD_DIR/DragonCodexBoot.app"
case "$mode" in
  run|--verify)
    open -n "$app_bundle" --args --preview "$@"
    if [[ "$mode" == --verify ]]; then
      sleep 1
      pgrep -f "$app_bundle/Contents/MacOS/DragonCodexBootMac" >/dev/null
    fi
    ;;
  --logs|--telemetry)
    open -n "$app_bundle" --args --preview "$@"
    /usr/bin/log stream --level info --style compact --predicate 'process == "DragonCodexBootMac"'
    ;;
  --debug)
    open -n "$app_bundle" --args --preview "$@"
    sleep 1
    preview_pid="$(pgrep -f "^$app_bundle/Contents/MacOS/DragonCodexBootMac( |$)" | head -1)"
    lldb -p "$preview_pid"
    ;;
  *)
    printf 'usage: %s [run|--verify|--logs|--telemetry|--debug] [preview options]\n' "$0" >&2
    exit 2
    ;;
esac
