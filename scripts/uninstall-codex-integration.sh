#!/usr/bin/env bash
set -euo pipefail

install_app="$HOME/Applications/DragonCodexBoot.app"
agent_label="community.dragoncodexboot.codex-watcher"
agent_plist="$HOME/Library/LaunchAgents/$agent_label.plist"
launch_domain="gui/$(id -u)"
app_executable="$install_app/Contents/MacOS/DragonCodexBootMac"
expected_bundle_id="community.dragoncodexboot.macos"

if [[ -e "$install_app" || -L "$install_app" ]]; then
  existing_bundle_id="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$install_app/Contents/Info.plist" 2>/dev/null || true)"
  if [[ -L "$install_app" || "$existing_bundle_id" != "$expected_bundle_id" ]]; then
    printf 'Refusing to execute or remove an unrelated app at: %s\n' "$install_app" >&2
    exit 1
  fi
fi

launchctl bootout "$launch_domain/$agent_label" >/dev/null 2>&1 || true

if [[ -x "$app_executable" ]]; then
  "$app_executable" --uninstall-integration
  rm -rf "$install_app"
  printf 'Removed Codex launch monitor and app: %s\n' "$install_app"
else
  rm -f "$agent_plist"
  printf 'Installed app was not found: %s\n' "$install_app"
fi

printf 'Configuration and copied video in ~/Library/Application Support/DragonCodexBoot were kept.\n'
