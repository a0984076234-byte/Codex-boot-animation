#!/bin/bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
"$repo_root/scripts/build-macos-standalone.sh"
version="$(tr -d '\r\n' < "$repo_root/VERSION")"
name="CodexBootAnimation-$version-macos-arm64-with-video.zip"
mkdir -p "$repo_root/dist"
ditto -c -k --sequesterRsrc --keepParent \
  "$repo_root/build/macos/Dragon Codex Boot.app" "$repo_root/dist/$name"
(cd "$repo_root/dist" && shasum -a 256 "$name" > "$name.sha256")
printf 'Packaged: %s\n' "$repo_root/dist/$name"
