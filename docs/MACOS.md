# macOS 版

macOS 版是 `macos/` 下的 Swift 原生启动器，要求 macOS 13 或更新版本以及 Swift 5.9 以上。直接打开启动器时，它使用系统 `AVKit` 播放本地 MP4、启动 `com.openai.codex`，并在可配置的等待点暂停。

## 从 Codex 图标启动动画

在仓库根目录构建并安装可选的启动监视器：

```sh
./scripts/install-codex-integration.sh
```

脚本把 `DragonCodexBoot.app` 安装到 `~/Applications/`，并在当前用户的 `~/Library/LaunchAgents/` 注册登录监视器。它会在当前会话启动，并在之后登录时自动运行；可在“系统设置 → 通用 → 登录项”停用。监视器只等待配置中 Codex Bundle Identifier 对应的新进程；从原 Codex 图标启动后，它等待 Codex 的可见主窗口出现，再按窗口边界播放动画。已运行的 Codex 被再次激活时不会重播。

动画窗口会与 Codex 窗口的位置和尺寸一致，使用 12 pt 连续圆角，四角透明并显示下方 Codex 窗口。视频等比例完整显示，宽高比不同时保留黑边，不裁掉人物。动画结束或按 `Esc` 后，焦点交还给 Codex。实现只读取系统提供的窗口位置和尺寸，不申请屏幕录制或辅助功能权限；不替换或修改官方 Codex App，也不接管 Dock 图标。

移除监视器和安装的启动器：

```sh
./scripts/uninstall-codex-integration.sh
```

这会保留 `~/Library/Application Support/DragonCodexBoot/` 中的配置和视频。若要再次使用，重新运行安装脚本即可。

## 功能边界

- 直接打开启动器时仍使用全屏播放；从 Codex 图标触发时使用 Codex 当前窗口的边界。
- 它不把真实 Codex 窗口实时裁切到动画里的屏幕矩形。Windows 版对应效果依靠 DWM 缩略图，macOS 没有相同的无权限接口。
- 启动器不会改写 Dock、Launch Services 或 Codex App。可选监视器只增加当前用户的 LaunchAgent，用户可在系统设置中停用或通过卸载脚本移除。
- `AppBundleIdentifier` 默认是 `com.openai.codex`。如果本机使用不同的发行渠道，可填写准确的 `AppPath`，例如 `/Applications/Codex.app`。
- 配置位于 `~/Library/Application Support/DragonCodexBoot/`；首次运行会从 App 包复制配置模板。MP4 在该目录下的 `media/startup.mp4`，也可在配置里填绝对路径。启动信息写入 macOS Console。
- 构建脚本会把 `media/startup.mp4` 和 `media/MEDIA_NOTICE.md` 一并装进 App；首次运行时复制到用户配置目录。

## 构建

在仓库根目录运行：

```sh
./scripts/build-macos.sh
```

生成的 App 位于 `build/DragonCodexBoot.app`。脚本使用 `swiftc` 和本机 SDK，无需安装第三方依赖。构建结果使用本机 ad hoc 签名，仅用于本机检查；公开分发还需要 Developer ID 签名与 notarization。

首次运行后，把自己的 H.264/AAC MP4 放到：

```text
~/Library/Application Support/DragonCodexBoot/media/startup.mp4
```

编辑同目录的 `launcher.json`，按视频修改 `HoldAt`、`TransitionStart`、`TransitionEnd`、`Volume`。如 Codex 未被自动找到，可填写应用 Bundle Identifier 或 `AppPath`。

## 原生暗色结尾

可在现有 JSON 中添加 `"OutroStyle": "darkNative"`。省略该字段或填写 `"original"` 时保留原片结尾，旧版配置仍可读取。

`darkNative` 专用于随附的约 14 秒视频：前 8 秒保留人物动画并使用强度 0.22 的亮度锐化；第 8 至 8.5 秒渐变到 `#181818`，随后隐藏视频画面，原音轨继续播放。第 8.5 至 9 秒显示当前目标应用的图标，以及 AppKit 原生文字“Codex / 开始构建 / 让想法成为现实。”。文字使用固定 pt 字号并随窗口更新 backing scale，避免将低清文字再次放大。`HoldAt` 至少为 8.5 秒；未就绪时停留在暗色等待画面。最终画面及音量使用同一淡出曲线。

原视频不会被修改或转码；原始 1080p 素材不会因此变为原生 4K。

## 安全预览与备份

Codex 的 Run 按钮调用以下入口，仅预览启动器，不打开、退出或显式激活目标 Codex：

```sh
./script/build_and_run.sh --verify
```

已有构建可直接独立预览：

```sh
open -n /private/tmp/dragon-codex-boot-local/DragonCodexBoot.app --args --preview
```

预览默认启用暗色结尾并静音。可选参数：`--preview-sound` 保留音轨；`--preview-original` 对照原版；`--preview-screen 0` 选择显示器；`--preview-windowed` 使用 960×620 窗口；`--preview-at 10` 停留在指定媒体时点（30 秒后退出）；`--preview-ready-after 15` 模拟启动等待；`--preview-timeout 3` 检查超时；`--preview-move-after 5` 将预览移至另一块屏幕。Esc 始终可退出预览。

`DRAGON_BUILD_DIR` 可以为构建、检查、安装和 Run 入口指定同一个输出目录。建议使用 `/private/tmp/` 下的规范真实路径，以免 Documents 的文件属性影响本地签名；不要将同一个模块缓存分别以 `/tmp` 与 `/private/tmp` 路径使用。

替换已安装版本前执行：

```sh
./scripts/backup-macos-installation.sh
```

脚本在 `~/Library/Application Support/DragonCodexBoot Backups/` 创建独立时间戳备份，包含 App、完整 Application Support、LaunchAgent、源码提交及差异、SHA-256 清单和 `Restore.command`。备份完成前会比对原文件并检查签名。双击备份中的 `Restore.command` 可恢复，恢复前会保留被替换版本；也可以设置 `DRAGON_RESTORE_ROOT` 为临时目录，验证恢复过程而不修改安装或 launchd。

## 验证

```sh
./scripts/test-macos.sh
```

该脚本检查 Swift 编译、配置校验、淡出曲线、App 与登录监视器清单、本机签名及随包媒体。它不注册登录监视器、不启动 Codex，也不验证窗口匹配和焦点交接；这些行为需要在安装了 Codex 的 macOS 桌面上手动确认。完整 Xcode 环境下也可运行 `swift test --package-path macos` 执行单元测试。

## 独立轻量版

PR #3 的独立实现保留于 `src/macos/`，不安装 LaunchAgent、不读取 `launcher.json`，使用单独的 `build-macos-standalone.sh` / `test-macos-standalone.sh`。打包入口为 `package-macos.sh`。输出 `build/macos/Dragon Codex Boot.app`，不会覆盖本页的 `build/DragonCodexBoot.app`。见 [独立版说明](MACOS-STANDALONE.md)。
