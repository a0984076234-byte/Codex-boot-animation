#!/bin/bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
"$repo_root/scripts/package-macos-standalone.sh"
app="$repo_root/build/macos/Dragon Codex Boot.app"
version="$(tr -d '\r\n' < "$repo_root/VERSION")"
archive="$repo_root/dist/CodexBootAnimation-$version-macos-arm64-with-video.zip"
temp_dir="$(mktemp -d)"
trap 'rm -rf "$temp_dir"' EXIT

xcrun swiftc -swift-version 5 -target "$(uname -m)-apple-macosx13.0" \
  -module-cache-path "$repo_root/build/macos/module-cache" \
  "$repo_root/src/macos/PlaybackProgressWatchdog.swift" \
  "$repo_root/tests/macos-watchdog/main.swift" -o "$temp_dir/watchdog-tests"
"$temp_dir/watchdog-tests"

plutil -lint "$app/Contents/Info.plist"
test "$(/usr/libexec/PlistBuddy -c 'Print CFBundleExecutable' "$app/Contents/Info.plist")" = DragonCodexBoot
lipo "$app/Contents/MacOS/DragonCodexBoot" -verify_arch arm64
cmp "$repo_root/media/startup.mp4" "$app/Contents/Resources/startup.mp4"
for notice in LICENSE THIRD_PARTY_NOTICES.md MEDIA_NOTICE.md; do
  test -s "$app/Contents/Resources/Licenses/$notice"
done
test -s "$app/Contents/Resources/AppIcon.icns"
unzip -tq "$archive"
ditto -x -k "$archive" "$temp_dir"
unpacked="$temp_dir/Dragon Codex Boot.app"
codesign --verify --strict "$unpacked"
cmp "$app/Contents/MacOS/DragonCodexBoot" "$unpacked/Contents/MacOS/DragonCodexBoot"
cmp "$repo_root/media/startup.mp4" "$unpacked/Contents/Resources/startup.mp4"
(cd "$repo_root/dist" && shasum -a 256 -c "$(basename "$archive").sha256")
printf 'PASS: build, arm64 binary, bundle, media, licenses, signature and archive round trip.\n'
printf 'Playback and application handoff require the manual checks in docs/MACOS-STANDALONE.md.\n'
