# 安装说明 / Installation

## Windows 10/11 x64

从 GitHub Releases 下载 Windows ZIP，解压到长期保留的位置，再启动控制中心。先导入或选择主题并点击“预览”；确认动画和 Codex 窗口衔接正常后，才点击“启用启动动画”。启用时控制中心会先创建备份，再接管可识别的 Codex 快捷方式。

停用会恢复安装记录中的原始入口。移动或删除程序目录前先停用；若控制中心无法启动，可运行解压目录中的 `Restore-Original-Entrypoints.cmd`。

Windows N 版本可能缺少系统媒体组件。首版不需要管理员权限，但修改公共开始菜单或其他用户目录的快捷方式可能因权限被跳过。

## macOS 13+ Apple Silicon

下载 macOS ZIP/DMG，拖入“应用程序”后打开控制中心。首版包没有 Apple 公证，首次打开可能需要在“系统设置 → 隐私与安全性”中允许打开。仅支持 Apple Silicon，Intel Mac 不在 V1 范围内。

控制中心只在用户明确启用后安装 LaunchAgent。停用会卸载 LaunchAgent 并保留主题和备份。macOS 预览可以在没有安装 Codex 的情况下运行；完整窗口交接需要本机存在可匹配的 Codex 应用。

## English

### Windows 10/11 x64

Download the Windows ZIP from GitHub Releases, extract it to a permanent folder, and open the control center. Import or select a theme and use **Preview** first. **Enable startup animation** creates a backup before taking over recognized Codex shortcuts.

Disable restores the recorded original entries. Before moving or deleting the folder, disable the integration. If the control center cannot start, run `Restore-Original-Entrypoints.cmd`.

Windows N may need the system media components. Administrator rights are not required for normal use, but public or other-user shortcut locations may be skipped when they are not writable.

### macOS 13+ Apple Silicon

Download the macOS ZIP/DMG and move the app to Applications. The V1 package is not notarized; Gatekeeper may require an explicit approval in System Settings. Intel Macs are out of scope for V1.

The control center installs a LaunchAgent only after explicit enable. Disable unloads it while retaining themes and backups. Preview works without Codex installed; full handoff requires a matching Codex app.
