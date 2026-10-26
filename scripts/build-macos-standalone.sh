#!/bin/bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
build_dir="$repo_root/build/macos"
app="$build_dir/Dragon Codex Boot.app"
source_dir="$repo_root/src/macos"
module_cache="$build_dir/module-cache"

if [[ "$(uname -s)" != Darwin ]]; then
  printf 'This build requires macOS and Xcode Command Line Tools.\n' >&2
  exit 1
fi
test -s "$repo_root/media/startup.mp4"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources/Licenses" "$module_cache" "$build_dir/AppIcon.iconset"
cp "$source_dir/Info.plist" "$app/Contents/Info.plist"
version="$(tr -d '\r\n' < "$repo_root/VERSION")"
/usr/libexec/PlistBuddy -c "Set CFBundleShortVersionString $version" "$app/Contents/Info.plist"
xcrun swiftc -swift-version 5 -O -target arm64-apple-macosx13.0 \
  -module-cache-path "$module_cache" \
  -framework AppKit -framework AVFoundation -framework QuartzCore \
  "$source_dir/PlaybackProgressWatchdog.swift" "$source_dir/main.swift" -o "$app/Contents/MacOS/DragonCodexBoot"
xcrun swift -module-cache-path "$module_cache" "$source_dir/icon.swift" "$build_dir/AppIcon.iconset"
cp "$build_dir/AppIcon.iconset.icns" "$app/Contents/Resources/AppIcon.icns"
cp "$repo_root/media/startup.mp4" "$app/Contents/Resources/startup.mp4"
cp "$repo_root/LICENSE" "$repo_root/THIRD_PARTY_NOTICES.md" "$repo_root/media/MEDIA_NOTICE.md" "$app/Contents/Resources/Licenses/"
cp "$repo_root/docs/MACOS-STANDALONE.md" "$app/Contents/Resources/"
codesign --force --sign - "$app"
codesign --verify --strict "$app"
printf 'Built: %s\n' "$app"
