#!/usr/bin/env bash
set -euo pipefail
# Keep the packaging entry introduced by PR #3; its standalone build is isolated.
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
exec "$repo_root/scripts/package-macos-standalone.sh" "$@"
