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

## Release

推送版本标签后，`release.yml` 会复用 Windows 和 macOS 构建工作流，等待测试和打包完成，再自动创建 GitHub Release 并上传桌面包：

```sh
git tag v0.2.1
git push origin v0.2.1
```

用户下载入口是仓库的 **Releases → Latest**。网站的“下载桌面版”按钮也指向这里。

Release 包含：

- Windows 控制中心便携 ZIP、NSIS 安装器；
- macOS Apple Silicon 控制中心 ZIP/DMG；
- 可单独导入的演示主题 ZIP；
- 每份下载对应的 SHA-256 文件。

安装包默认只预览，不会自动接管系统入口。Release 说明必须明确这一点，并引导用户在控制中心中明确点击“启用启动动画”。

若两平台构建成功但发布阶段失败，可运行 **Publish verified build**，输入原标签与 Release run ID。流程会检查 run 提交与标签完全一致、两平台测试均成功，再复用原产物发布，不移动标签，也不需要重复编译。Windows ZIP 路径会统一为 POSIX 分隔符后校验和打包。

`build.yml` 会构建、运行离线检查、打包仓库中的示例视频，并把 ZIP 与哈希作为 Actions artifact 保留；它不自动建立 Release，也不会读取开发者的个人视频目录。

## 发布内容边界

- 源码、配置模板和本项目文档使用 MIT 许可证。
- 默认 ZIP（`CodexBootAnimation-版本-win-x64-with-video.zip`）包含程序、示例配置、完整1080p龙娘启动视频、导入及入口替换脚本、一键恢复/重新扫描、许可证和文档。控制中心首次运行只预览；用户点击“启用启动动画”后才接管入口，必须在 Release 说明中明确。
- 原角色图、其他制作中间文件、提取的官方图标、本机路径与日志没有纳入源码仓库。
- 视频放在 `media/startup.mp4`，素材说明放在 `media/MEDIA_NOTICE.md`；代码 MIT 许可不自动扩展到角色、音乐和第三方标志。

## English

The repository is already created and points to `a0984076234-byte/Codex-boot-animation`. Do not run `gh repo create` again. Verify the remote with `git remote -v`, then create a version tag to publish a release:

```sh
git tag v0.2.1
git push origin v0.2.1
```

The release workflow uploads Windows and macOS desktop packages. The control center previews by default and only backs up and changes recognized Codex launch entry points after the user explicitly chooses **Enable startup animation**. Code is MIT-licensed; bundled demonstration media has a separate notice and may carry different rights.
