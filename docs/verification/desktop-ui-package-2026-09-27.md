# MauLink 桌面 UI 与本地打包记录

**日期：** 2026-09-27  
**状态：** 首版产品界面已接入，macOS arm64 `.app` 与 ZIP 已生成；DMG 和交互式桌面验收未完成。

## 本轮交付

- 主窗口改用 `frontend/` 正式页面，包含服务器与分组列表、搜索、新增/编辑/删除、密码或私钥认证、连接测试、Host Key 确认、认证提示及基础 Shell 工作区。
- 私钥通过原生文件选择器取得短期文件 token；页面不直接获取本地路径。敏感凭据沿用系统凭据管理后端。
- 终端页面使用本地 vendored xterm.js，支持 VT 控制序列、滚动缓冲、键盘原样输入和 Fit 调整；前端在 xterm 解析回调完成后才发送累计 ACK。
- 主 Tauri 配置将窗口指向正式页面，添加内容安全策略、产品窗口尺寸与应用元数据，并为 macOS/Windows 包接入统一图标。IPC harness 留在单独配置中。
- 正式 CSP 仍禁止内联脚本和外部连接；样式策略允许 inline style，因为 xterm.js DOM renderer 在运行时创建主题和尺寸样式节点。

## 验证

- `node --check` 对主页面脚本、终端输出队列、测试文件和 vendored xterm.js 通过；`node --test frontend/tests/terminal-codec.test.mjs`：4 项通过，覆盖原始 UTF-8/VT 字节、解析后 ACK、长度错误和序号缺口拒绝。
- 前端 DOM id 无重复、脚本引用的元素均存在；调用的 Tauri commands 均已注册并授予正式 capability。
- `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets --locked -- -D warnings`、`cargo check --workspace --all-targets --locked` 通过。
- `cargo test --workspace --locked` 通过：38 项通过，4 项按需运行的原生凭据/OpenSSH 集成测试被忽略。
- Tauri release 编译通过，产物为 arm64 Mach-O `.app`；`Info.plist` 指向 `Contents/Resources/icon.icns`。检查 release 二进制可见打包后的前端入口和 `terminal-codec.mjs` 资源路径。
- 使用 `ditto` 将 `.app` 打成 ZIP，`unzip -t` 检查压缩包完整性通过。

## 未完成与限制

- `cargo tauri build --bundles app dmg --no-sign` 的应用编译和 `.app` bundle 成功；DMG 阶段的 `hdiutil create` 返回 `设备未配置`。本机 `diskutil list` 同时报无法使用 DiskManagement framework，因此当前环境没有生成 DMG。可用的本地包是 `target/release/bundle/macos/MauLink.app` 和 `target/release/bundle/macos/MauLink_0.1.0_aarch64.zip`。
- 本轮按用户选择跳过真实 Tauri IPC 压测，没有吞吐和往返延迟数据；也没有运行桌面应用做视觉或端到端交互验收。
- 当前 ZIP 是本机 arm64、非 Developer ID 签名及公证的构建，仅证明可编译和打包，不可视为面向公众发布的安装包。Windows 包和 Windows 真机验收未做。
- 尚未在真实 Tauri 窗口内验证 IME、复制粘贴、全屏 TUI、窗口缩放和跨多个 Tab 的视觉表现；M4 IPC 性能验收仍需真实测量。
