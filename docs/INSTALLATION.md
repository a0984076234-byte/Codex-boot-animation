# 安装说明 / Installation

## 从网站到桌面

1. 在网站预览主题，点击对应系统的“下载 Windows 版”或“下载 macOS 版”。
2. 桌面包已附带默认 Dragon 主题和视频，不需要另找 MP4，也不需要 Node.js 或 Rust。
3. 安装并打开 **Codex Boot Animation**。先预览默认主题，再明确点击“启用启动动画”。启用前自动创建备份。
4. 退出 Codex 后重新打开，检查动画交接。Mac 上仅激活已有的 Codex 进程不会重播。
5. 停用、更换主题或卸载时，先在控制中心点击“停用启动动画”。

网站不修改系统入口；网页里的选择和编辑不会自动同步到桌面。自选 GitHub 主题需要在桌面控制中心再次导入同一仓库地址。演示主题 ZIP 解压后，使用“导入本地主题”选择含 `codex-theme.json` 的目录。主题 ZIP 本身不是可执行应用。

下载与校验文件都在 [GitHub Releases](https://github.com/a0984076234-byte/Codex-boot-animation/releases/latest)。

## Windows 10/11 x64

从 GitHub Releases 下载 Windows 控制中心 ZIP 或 NSIS 安装器。便携 ZIP 请解压到长期保留的位置，再启动控制中心。先导入或选择主题并点击“预览”；确认动画和 Codex 窗口衔接正常后，才点击“启用启动动画”。启用时控制中心会先创建备份，再接管可识别的 Codex 快捷方式。

便携包主程序名为 `codex-boot-animation-control-center.exe`。同目录的 `native/` 和 `demo/` 必须完整保留。

停用会恢复安装记录中的原始入口。移动或删除程序目录前先停用。已启用时若控制中心无法启动，可在 `%APPDATA%\community.codexbootanimation.controlcenter\native\windows\DragonCodexBoot\` 中运行 `Restore-Original-Entrypoints.cmd`，该目录对应实际保存集成记录的引擎副本。

Windows N 版本可能缺少系统媒体组件。首版不需要管理员权限，但修改公共开始菜单或其他用户目录的快捷方式可能因权限被跳过。

## macOS 13+ Apple Silicon

下载 macOS Apple Silicon ZIP/DMG，拖入“应用程序”后打开控制中心。首版包没有 Apple 公证，首次打开可能需要在“系统设置 → 隐私与安全性”中允许打开。仅支持 Apple Silicon，Intel Mac 不在 V1 范围内。

控制中心只在用户明确启用后安装 LaunchAgent。停用会卸载 LaunchAgent 并保留主题和备份。macOS 预览可以在没有安装 Codex 的情况下运行；完整窗口交接需要本机存在可匹配的 Codex 应用。

## English

### From website to desktop

Preview on the website, then download the Windows control-center ZIP/installer or macOS arm64 DMG/ZIP. The demo theme and video are already included; Node.js and Rust are not required. Open **Codex Boot Animation**, preview the theme, then explicitly enable integration. A backup is created first. Quit and reopen Codex to test; disable integration before uninstalling or changing themes.

Browser selections and edits do not sync to desktop. Import the same public GitHub URL again in the desktop app, or extract a theme ZIP and choose its manifest folder through **Import local theme**. A theme ZIP is not an executable.

### Windows 10/11 x64

Download the Windows control-center ZIP or NSIS installer from GitHub Releases. Extract the portable ZIP to a permanent folder, then open the control center. Import or select a theme and use **Preview** first. **Enable startup animation** creates a backup before taking over recognized Codex shortcuts.

The portable executable is `codex-boot-animation-control-center.exe`; retain the adjacent `native/` and `demo/` directories. Disable restores the recorded original entries. Before moving or deleting the folder, disable integration. If the control center cannot start after enabling, use `Restore-Original-Entrypoints.cmd` in `%APPDATA%\community.codexbootanimation.controlcenter\native\windows\DragonCodexBoot\`.

Windows N may need the system media components. Administrator rights are not required for normal use, but public or other-user shortcut locations may be skipped when they are not writable.

### macOS 13+ Apple Silicon

Download the macOS Apple Silicon ZIP/DMG and move the app to Applications. The V1 package is not notarized; Gatekeeper may require an explicit approval in System Settings. Intel Macs are out of scope for V1.

The control center installs a LaunchAgent only after explicit enable. Disable unloads it while retaining themes and backups. Preview works without Codex installed; full handoff requires a matching Codex app.
