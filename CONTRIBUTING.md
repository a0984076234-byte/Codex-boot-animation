# 贡献说明

欢迎修复媒体播放、窗口定位、DPI、配置校准、主题导入和无障碍交接方面的问题。

1. Fork 仓库，在分支中修改源码。
2. Windows 变更运行 `scripts/test.ps1`；macOS 变更运行 `swift test --package-path macos` 和 `scripts/test-macos.sh`。控制中心变更运行 `cd control-center && npm install && npm run build`，有 Rust 工具链时再运行 `cargo test --manifest-path src-tauri/Cargo.toml`。涉及登录监视器、窗口定位、圆角裁切或焦点交接的变动，再在安装了 Codex 的 macOS 桌面上手动检查。
3. 提交 PR，写明具体触发条件、变化后的表现和验证范围。

提交代码适用仓库的 MIT 许可证。不要提交聊天数据、账户文件、凭据、个人配置、官方提取资产或许可不明的角色和音乐文件。主题媒体必须保留来源和单独许可说明。

几何与配置变更应补充能发现真实回归的测试。媒体或桌面行为不能只用离线几何检查作为验证。
