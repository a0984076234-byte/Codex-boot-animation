#!/usr/bin/env python3
"""Select complete desktop packages; never publish the frontend build tree."""
import hashlib
import os
import pathlib
import shutil
import sys
import zipfile

repo = pathlib.Path(os.environ.get("RELEASE_SOURCE_ROOT", pathlib.Path(__file__).resolve().parent.parent))
version = (repo / "VERSION").read_text().strip()
source, destination = map(pathlib.Path, sys.argv[1:])
required = (
    f"CodexBootAnimation-{version}-win-x64-control-center.zip",
    f"CodexBootAnimation-{version}-macos-arm64.zip",
    f"CodexBootAnimation-{version}-macos-arm64.dmg",
)
assets = []
for name in required:
    matches = list(source.rglob(name))
    if len(matches) != 1:
        raise SystemExit(f"Expected exactly one {name}, found {len(matches)}")
    assets.append(matches[0])
installers = list(source.rglob("*-setup.exe"))
if len(installers) != 1:
    raise SystemExit(f"Expected one Windows installer, found {len(installers)}")
assets.extend(installers)
windows = assets[0]
with zipfile.ZipFile(windows) as archive:
    names = [name.replace("\\", "/") for name in archive.namelist()]
    if len(set(names)) != len(names):
        raise SystemExit("Duplicate Windows archive paths")
    if any(name.startswith("/") or ".." in name.split("/") for name in names):
        raise SystemExit("Unsafe Windows archive path")
    for name in (
        "codex-boot-animation-control-center.exe",
        "native/windows/DragonCodexBoot/DragonCodexBoot.exe",
        "native/windows/DragonCodexBoot/scripts/integrate-entrypoints.ps1",
        "demo/codex-theme.json",
        "demo/media/startup.mp4",
    ):
        if name not in names:
            raise SystemExit(f"Missing Windows portable resource: {name}")
destination.mkdir(parents=True, exist_ok=True)
for asset in assets:
    # GitHub replaces spaces in asset names; hash files must use that same name.
    target = destination / asset.name.replace(" ", ".")
    if asset == windows:
        # Windows PowerShell 5 Compress-Archive uses backslashes inside ZIPs.
        with zipfile.ZipFile(asset) as original, zipfile.ZipFile(target, "w", zipfile.ZIP_DEFLATED) as normalized:
            for entry in original.infolist():
                path = entry.filename
                entry.filename = path.replace("\\", "/")
                normalized.writestr(entry, original.read(path))
    else:
        shutil.copy2(asset, target)
theme = destination / "CodexBootAnimation-demo-theme.zip"
with zipfile.ZipFile(theme, "w", zipfile.ZIP_DEFLATED) as archive:
    for relative in (
        "codex-theme.json", "media/startup.mp4", "media/poster.jpg", "media/MEDIA_NOTICE.md"
    ):
        archive.write(repo / relative, relative)
for asset in sorted(destination.iterdir()):
    if asset.suffix == ".sha256":
        continue
    digest = hashlib.sha256(asset.read_bytes()).hexdigest()
    asset.with_name(asset.name + ".sha256").write_text(f"{digest}  {asset.name}\n")
    print(f"{asset.name}: {asset.stat().st_size} bytes")
