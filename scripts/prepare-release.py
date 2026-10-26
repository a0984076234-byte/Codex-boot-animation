#!/usr/bin/env python3
"""Select complete desktop packages; never publish the frontend build tree."""
import hashlib
import pathlib
import shutil
import sys
import zipfile

repo = pathlib.Path(__file__).resolve().parent.parent
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
    for name in (
        "codex-boot-animation-control-center.exe",
        "native/windows/DragonCodexBoot/DragonCodexBoot.exe",
        "native/windows/DragonCodexBoot/scripts/integrate-entrypoints.ps1",
        "demo/codex-theme.json",
        "demo/media/startup.mp4",
    ):
        if name not in archive.namelist():
            raise SystemExit(f"Missing Windows portable resource: {name}")
destination.mkdir(parents=True, exist_ok=True)
for asset in assets:
    shutil.copy2(asset, destination / asset.name)
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
