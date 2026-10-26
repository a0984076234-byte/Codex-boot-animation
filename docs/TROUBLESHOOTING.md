# 故障排查 / Troubleshooting

## 预览没有修改系统

预览使用临时 `launcher.json`，不会写入正式配置，也不会接管快捷方式或安装 LaunchAgent。若预览失败，先检查主题的视频路径、格式和媒体组件。

## 启用失败

控制中心会在启用前创建备份，并在原生引擎或入口替换失败时尝试回滚。打开“设置 → 诊断”查看用户数据目录；保留 `launcher-backup.json` 后再运行恢复。

## 主题导入失败

主题仓库必须公开可访问，根目录必须有 `codex-theme.json`。视频、封面和许可说明只能使用主题目录内的相对路径；压缩包中的脚本不会执行，符号链接、路径穿越和超限媒体会被拒绝。

## macOS 打不开

V1 macOS 包暂未公证。到“系统设置 → 隐私与安全性”允许打开，并确认系统为 macOS 13+ Apple Silicon。没有 Codex 时可以继续验证视频预览，但不能验证真实窗口交接。

## English

### Preview changed nothing

Preview uses a temporary `launcher.json`. It does not write the installed configuration, replace shortcuts, or install a LaunchAgent. Check the theme media path, codec, and system media components if playback fails.

### Enable failed

The control center creates a backup before enabling and attempts to roll back when the native engine or entrypoint integration fails. Use **Settings → Diagnostics** to locate the user data directory, keep `launcher-backup.json`, and run Restore.

### Theme import failed

The repository must be public and its root must contain `codex-theme.json`. Video, poster, and media notice paths must stay relative to the theme directory. Archive scripts are never executed; symlinks, path traversal, and oversized media are rejected.

### macOS will not open

The V1 macOS package is not notarized. Allow it in System Settings → Privacy & Security and confirm macOS 13+ on Apple Silicon. Preview works without Codex, but real window handoff cannot be tested without it.
