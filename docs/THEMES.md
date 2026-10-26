# Codex Boot Animation 主题规范

## Theme manifest

每个主题仓库的根目录必须包含 `codex-theme.json`。V1 只接受公开 GitHub 仓库，并且只读取声明式清单和媒体文件，不执行仓库内的脚本。

```json
{
  "schemaVersion": 1,
  "id": "example-theme",
  "name": "Example Theme",
  "version": "1.0.0",
  "description": "A short description.",
  "video": "media/startup.mp4",
  "poster": "media/poster.png",
  "license": "CC BY 4.0",
  "source": "https://github.com/example/codex-theme",
  "mediaNotice": "media/MEDIA_NOTICE.md",
  "timeline": {
    "holdAt": 11.3,
    "transitionStart": 12.7,
    "transitionEnd": 13.65,
    "maxWaitSeconds": 60,
    "volume": 1,
    "screenFrames": [
      {
        "time": 12.65,
        "x": 0.123,
        "y": 0.094,
        "width": 0.736,
        "height": 0.725
      },
      {
        "time": 13.5,
        "x": 0,
        "y": 0,
        "width": 1,
        "height": 1
      }
    ]
  }
}
```

## 安全边界

- `video`、`poster` 和 `mediaNotice` 必须是主题目录内的相对路径，不能包含 `..`、绝对路径或符号链接跳出主题目录。
- 视频最大 250 MB，封面最大 10 MB，单个媒体文件最大 250 MB。
- 只接受 MP4、WebM、MOV、PNG、JPG、JPEG、WebP 和 Markdown 许可说明。
- 时间线必须满足 `0 < holdAt < transitionStart < transitionEnd`，关键帧时间严格递增，矩形坐标在 `[0, 1]` 内。
- GitHub 导入使用公开 `https://github.com/<owner>/<repo>` 地址；不会读取 token，也不会访问私有仓库。
- 控制中心在导入前校验清单、路径、媒体类型和文件大小；校验失败不会安装主题。V1 不提供媒体哈希签名或作者身份认证。

## 本地主题

用户可以从控制中心导入一个包含 `codex-theme.json` 的本地目录。原目录不会被修改，主题会被复制到平台用户数据目录。删除主题只删除其副本，不会删除用户选择的原文件。

主题详情提供时间线表单，可编辑停留点、过渡开始/结束、最长等待、音量和屏幕关键帧。保存只更新已导入的主题副本；启用前可先预览检查效果。

## 兼容旧配置

原有 `config/launcher.example.json` 和 `config/launcher.macos.example.json` 继续作为原生引擎配置模板。控制中心把主题时间线转换为对应平台的 `launcher.json`，不改变旧字段含义。
