#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
backup_root="${1:-$HOME/Library/Application Support/DragonCodexBoot Backups/$(date +%Y%m%d-%H%M%S)}"
installed_app="$HOME/Applications/DragonCodexBoot.app"
support_root="$HOME/Library/Application Support/DragonCodexBoot"
agent_plist="$HOME/Library/LaunchAgents/community.dragoncodexboot.codex-watcher.plist"

if [[ -e "$backup_root" ]]; then
  printf 'Backup destination already exists: %s\n' "$backup_root" >&2
  exit 1
fi
for item in "$installed_app" "$support_root" "$agent_plist"; do
  [[ -e "$item" ]] || { printf 'Missing installation item: %s\n' "$item" >&2; exit 1; }
done

mkdir -p "$backup_root"
ditto "$installed_app" "$backup_root/DragonCodexBoot.app"
ditto "$support_root" "$backup_root/ApplicationSupport"
cp "$agent_plist" "$backup_root/watcher.plist"
cp "$repo_root/scripts/restore-macos-installation.command" "$backup_root/Restore.command"
chmod +x "$backup_root/Restore.command"
git -C "$repo_root" rev-parse HEAD > "$backup_root/source-revision.txt"
git -C "$repo_root" status --short > "$backup_root/source-status.txt"
git -C "$repo_root" diff --binary HEAD > "$backup_root/source-changes.patch"
git -C "$repo_root" archive --format=tar HEAD > "$backup_root/source.tar"

diff -qr "$installed_app" "$backup_root/DragonCodexBoot.app"
diff -qr "$support_root" "$backup_root/ApplicationSupport"
cmp "$agent_plist" "$backup_root/watcher.plist"
codesign --verify --deep --strict "$backup_root/DragonCodexBoot.app"
(
  cd "$backup_root"
  find . -type f ! -name SHA256SUMS -print0 | sort -z | xargs -0 shasum -a 256 > SHA256SUMS
  shasum -a 256 --check SHA256SUMS
)
printf 'Verified backup: %s\n' "$backup_root"
