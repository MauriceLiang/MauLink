# MauLink 应用内更新

## 当前状态

截至 2026-10-12，设置中的更新 UI、Tauri 插件注册、Mock Harness、下载状态机和安装前安全门禁已经实现。真实在线更新**尚未启用**：应用没有更新 HTTPS endpoint 或受信公钥配置；原有 `v0.1.1-rc.7` 安装包也不含本次新增的 updater runtime，无法通过应用内更新升级。启用前需要先手动安装首个带 updater 的完整安装包。

目前不创建更新索引、不签名更新归档、不更改发布工作流、不发布 Release 或 Tag。缺少更新地址和公钥时，桌面应用会显示“此版本尚未配置受信任的更新来源”；浏览器 Harness 使用本地 Mock，绝不会调用真实 updater 或外部更新服务。

## 已实现部分

- 依赖：Rust `tauri-plugin-updater = 2.12.0`、`tauri-plugin-process = 2.3.1`；前端对应插件 `@tauri-apps/plugin-updater = 2.12.0`、`@tauri-apps/plugin-process = 2.3.1`。版本固定并由 lockfile 管理。
- 注册与权限：插件由 `src-tauri/src/lib.rs` 注册；`src-tauri/capabilities/main.json` 只添加检查、下载、安装和重启所需的权限，没有授予进程退出或合并式下载安装权限。
- UI：设置 → 通用显示软件更新区；关于窗口复用同一控制器跳转到该区。用户手动检查、查看说明、下载，并明确选择安装。
- 状态与资源：控制器负责唯一更新资源、串行操作、进度增量、未知大小进度显示、稳定错误类型和关闭/卸载时的资源释放。更新数据不写入 `AppSettings`。
- 安全门禁：有任何上传/下载任务或传输尚在启动时禁止安装；所有非终态 SSH 连接和终端会先提示风险，需用户确认断开并等待 Core 快照进入终态。安装期间阻止新的连接和传输；设置已有自动保存事务未成功时取消安装。
- Harness：`/?harness=visual&page=settings-general&update=available` 可查看“发现版本 → 查看说明 → 下载 → 稍后安装/确认安装”流程。其他 `update` 场景为 `up-to-date`（默认）、`network-error`、`invalid-index`、`unsupported`。Harness 的 adapter 仅记录 Mock 调用。

## 启用正式更新前负责人必须完成

1. 确认公开 Stable 与 Preview 索引的 HTTPS 托管地址、访问地区和维护责任人；确认 Pages 可用，或提供负责人控制的可信静态 HTTPS 地址。Stable 与 Preview 必须使用独立索引。
2. 负责人在可信设备上按 [Tauri Updater 官方说明](https://v2.tauri.app/plugin/updater/)生成 updater minisign 密钥。只将公钥文本提供给应用配置；私钥及口令由负责人留在安全位置并独立备份，不能提交仓库或发送到聊天。
3. 明确受保护的 GitHub Environment / Secrets，仅给经过审批的发布工作流签名步骤授权。PR 和 Packaging Smoke 不得读取这些密钥。
4. 实施并测试平台更新归档、`.sig` 生成、三平台 metadata、索引防回退和发布顺序。只有 Release 公开且校验完成后，才能把新版本写入对应索引。
5. 首个带 updater 的版本需完整手动安装。随后用另一个更高版本分别验证 macOS ARM64、macOS Intel（无设备时标记 BLOCKED）和 Windows x64 的下载、签名拒绝、安装/重启接管及本地服务器资料与凭据保留。
6. 完成上述决策和测试后，再增量修改 `src-tauri/tauri.conf.json`、Windows MSI 配置合并逻辑及发布 workflows；不要提交密钥占位符或未经验证的正式 endpoint。

Tauri updater minisign 密钥与 Apple Developer ID、公证证书、Windows Authenticode 证书彼此独立。配置 updater 签名不表示安装包已获操作系统可信签名或公证。密钥遗失会使既有客户端无法信任使用新密钥签名的更新；轮换前必须设计客户端迁移版本或其他恢复机制。

## 本地检查与手动验收

基础检查：

```bash
npm --prefix frontend run type-check
npm --prefix frontend run test
npm --prefix frontend run build
cargo fmt --all -- --check
cargo check -p maulink-desktop --locked
```

检查设置页中英文、明暗主题和窄窗口布局；逐一选择上述 Harness 场景；验证未知下载总量不会显示虚假百分比、连续点击不会并发检查/下载、稍后安装会保留已下载状态。连接建立中或存在 SSH 会话时检查安装门禁；有活动传输时确认不会断开会话或启动安装。真机安装升级必须分别记录操作系统、CPU 架构、源/目标版本、应用退出/重启结果、更新签名结果及本地数据留存情况。Browser Harness、cargo check 和打包成功均不等于真实更新通过。

## 交付门禁

| 阶段 | 状态 | 说明 |
| --- | --- | --- |
| P0 依赖兼容性与基线 | PASS | 选择与当前 Tauri/Rust 工具链兼容的锁定版本；基线和改动后的检查结果在当前交付记录中报告。 |
| P1 插件与最小权限 | PARTIAL | 插件注册及权限已实现；无密钥编译可由本机打包和 Packaging Smoke 验证。真实启动/退出和三平台 smoke 仍需设备或 Actions 结果。 |
| P2 UI、状态机与安全门禁 | PASS WITH MOCK | 自动测试与 Browser Harness 验证本地 Mock 流程；不代表在线 endpoint 或安装包签名已验证。 |
| P3 HTTPS 更新索引 | BLOCKED | 等待负责人确认并提供 Stable/Preview 托管地址。 |
| P4 签名发布流水线 | BLOCKED | 等待负责人安全生成密钥、提供公钥文本并配置受保护的 Secrets/Environment。 |
| P5 跨版本真机安装 | BLOCKED | 需要两个全新递增版本以及 macOS ARM64、Intel 和 Windows x64 真机。 |
| P6 文档和交付 | IN PROGRESS | 本文及 `RELEASING.md` 已更新；代码交付后还需由负责人创建 PR 并查看 Actions。 |

发布责任和已有 DMG/MSI、`SHA256SUMS`、Draft 流程见 [RELEASING.md](RELEASING.md)。
