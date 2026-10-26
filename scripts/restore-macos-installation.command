#!/usr/bin/env bash
set -euo pipefail

backup_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
destination_root="${DRAGON_RESTORE_ROOT:-$HOME}"
installed_app="$destination_root/Applications/DragonCodexBoot.app"
support_root="$destination_root/Library/Application Support/DragonCodexBoot"
agent_plist="$destination_root/Library/LaunchAgents/community.dragoncodexboot.codex-watcher.plist"
agent_service="gui/$(id -u)/community.dragoncodexboot.codex-watcher"
(
  cd "$backup_root"
  shasum -a 256 --check SHA256SUMS
)
codesign --verify --deep --strict "$backup_root/DragonCodexBoot.app"

# An alternate root exercises the complete copy and verification without touching launchd.
if [[ "$destination_root" == "$HOME" ]]; then
  launchctl bootout "$agent_service" >/dev/null 2>&1 || true
  for attempt in {1..25}; do
    if ! launchctl print "$agent_service" >/dev/null 2>&1; then break; fi
    sleep 0.2
  done
fi
mkdir -p "$(dirname "$installed_app")" "$(dirname "$support_root")" "$(dirname "$agent_plist")"
for destination in "$installed_app" "$support_root"; do
  if [[ -e "$destination" ]]; then
    mv "$destination" "${destination}.before-restore-$(date +%Y%m%d-%H%M%S)"
  fi
done
ditto "$backup_root/DragonCodexBoot.app" "$installed_app"
ditto "$backup_root/ApplicationSupport" "$support_root"
cp "$backup_root/watcher.plist" "$agent_plist"
diff -qr "$backup_root/DragonCodexBoot.app" "$installed_app"
diff -qr "$backup_root/ApplicationSupport" "$support_root"
cmp "$backup_root/watcher.plist" "$agent_plist"
codesign --verify --deep --strict "$installed_app"
if [[ "$destination_root" == "$HOME" ]]; then
  for attempt in {1..3}; do
    if launchctl bootstrap "gui/$(id -u)" "$agent_plist"; then break; fi
    if [[ "$attempt" == 3 ]]; then exit 1; fi
    sleep 1
  done
fi
printf 'Restored and verified: %s\n' "$installed_app"
