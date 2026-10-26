# 发布到 GitHub

仓库：[`a0984076234-byte/Codex-boot-animation`](https://github.com/a0984076234-byte/Codex-boot-animation)，可见性：Public，默认分支：`main`。

项目公开名称为 **Codex Boot Animation**。`DragonCodexBoot*` 只作为兼容旧安装的原生内部文件名保留。

## 上传源码

本仓库源码已经独立整理。只上传 Git 跟踪文件，避免把原制作项目、下载目录和本机配置一起上传。

仓库已经创建并绑定到当前远程地址。只有在从零创建另一个副本时才需要使用 `gh repo create`；不要对当前仓库重复执行创建命令。

若要检查远程：

```sh
git remote -v
```

本项目的首次发布流程是：

```sh
git add .
git commit -m "feat: establish Codex Boot Animation v1"
git push -u origin main
```

## 首次 Release

1. 推送源码后，检查 GitHub Actions 的 Windows 构建结果。
2. 执行 `scripts/test.ps1` 和 `scripts/package.ps1`，确认程序包包含随附视频与媒体说明。
3. 上传版本包和对应 `.sha256` 到 Release。
4. Release 说明写明视频已经包含，解压即可运行，并说明如何换成自己的动画。

`build.yml` 会构建、运行离线检查、打包仓库中的示例视频，并把 ZIP 与哈希作为 Actions artifact 保留；它不自动建立 Release，也不会读取开发者的个人视频目录。

## 发布内容边界

- 源码、配置模板和本项目文档使用 MIT 许可证。
- 默认 ZIP（`CodexBootAnimation-版本-win-x64-with-video.zip`）包含程序、示例配置、完整1080p龙娘启动视频、导入及入口替换脚本、一键恢复/重新扫描、许可证和文档。控制中心首次运行只预览；用户点击“启用启动动画”后才接管入口，必须在 Release 说明中明确。
- 原角色图、其他制作中间文件、提取的官方图标、本机路径与日志没有纳入源码仓库。
- 视频放在 `media/startup.mp4`，素材说明放在 `media/MEDIA_NOTICE.md`；代码 MIT 许可不自动扩展到角色、音乐和第三方标志。

## English

The repository is already created and points to `a0984076234-byte/Codex-boot-animation`. Do not run `gh repo create` again. Verify the remote with `git remote -v`, then commit and push the first public baseline:

```sh
git add .
git commit -m "feat: establish Codex Boot Animation v1"
git push -u origin main
```

The control center previews by default. It only backs up and changes recognized Codex launch entry points after the user explicitly chooses **Enable startup animation**. Code is MIT-licensed; bundled demonstration media has a separate notice and may carry different rights.
