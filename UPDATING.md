# MauLink 应用内更新

## 当前状态

本分支在现有应用身份下实现 Stable / Preview 渠道偏好、版本规则、Rust 更新协调器和浏览器 Mock 流程；应用版本保持 `0.1.1-rc.8`。`src-tauri/tauri.conf.json` 目前没有更新公钥或 Stable / Preview HTTPS 索引，因此本机及已发布客户端都不能在线更新，也不会注册 updater 插件。首次启用在线更新前，必须先发布并手动安装一个带受信更新配置的完整应用包。

本次没有创建索引、更新归档或签名文件，也没有修改正式 Release 工作流、版本号、Tag、Release 或 PR。正式 Release 的通用 DMG/MSI 流程仍由 [RELEASING.md](RELEASING.md) 负责。

## 版本和渠道规则

- Stable 只接受严格 `X.Y.Z`；Preview 只接受严格 `X.Y.Z-rc.N`，`N` 为正整数。拒绝前缀 `v`、前导零、其他预发布标识和 `+build` 元数据。
- 候选版本采用 SemVer 顺序比较：例如 `0.1.1-rc.8 < 0.1.1 < 0.1.2-rc.1`，`rc.10 > rc.9`。
- `AppSettings.updateChannel` 是兼容旧设置的可空字段：`null` 按当前二进制版本推断默认渠道，用户明确选择后持久化。已安装版本始终来自应用程序，不由渠道下拉框改变。
- Rust 仅从受信配置中按固定顺序选择索引：第一个 HTTPS endpoint 为 Stable，第二个为 Preview。必须恰有两个不同的 HTTPS endpoint 和非空公钥；前端不传 URL。
- HTTP 404、410、204 返回 `channelEmpty`；权限错误、网络错误、无效 metadata 分开处理。Native 预检会先发一个 GET 以区分 HTTP 状态，之后 Tauri updater 会再次请求索引；配置托管时应允许这两次请求。
- 候选必须是所选渠道对应的严格版本。高于当前版本才会进入下载；相同版本显示已是最新；低于当前版本在 Rust 检查阶段即阻断，不下载、不安装。

## 已实现

- `tools/release-channel.mjs` 提供严格版本解析、Stable/Preview 分类和 SemVer 比较；测试覆盖 rc.9/rc.10、Stable、下一 patch、非法版本和大序号。
- `AppSettings.updateChannel`、TypeScript 合约及旧 JSON 兼容已接入。Windows MSI 构建 overlay 显式写入 `bundle.windows.allowDowngrades=true`，仍使用既有 `toMsiProductVersion` 映射。
- 前端通过受控 Tauri 命令检查、下载、安装和取消；JS updater 插件依赖与权限已移除。保留用户手动安装和已有连接/终端/SFTP 安全门禁。
- 更新设置分别展示已安装渠道和接收渠道。Preview 显示不稳定提示；切换只更新订阅偏好，不会立即改变已安装版本。
- Mock 场景：`up-to-date`、`available`、`stable-to-preview-upgrade`、`downgrade`、`channel-empty`、`network-error`、`invalid-index`、`signature-error`、`unsupported`。Harness 场景只在浏览器本地运行，不访问更新服务。

## 降级安全状态

SQLite 当前 schema 为 7，但项目尚无按发行版本记录的兼容性策略，且未完成 MSI 覆盖降级或 macOS `.app` 替换实机验证。因此 Rust 对所有 SemVer 降级一律拒绝，前端明确显示当前版本和目标版本。此阶段不提供降级二次确认或自动备份，因为尚无可验证的安全迁移规则；不能把 `allowDowngrades=true` 解释为降级已安全。直接使用完整 MSI 安装旧版仍需负责人先备份数据并按平台验收。

系统更新签名校验、HTTPS/TLS 校验没有关闭。应用 identifier、产品名、数据目录、SQLite 路径、凭据边界和 Windows MSI UpgradeCode 未变更。Tauri updater minisign 私钥与 Apple Developer ID、公证和 Windows Authenticode 证书相互独立。

## 启用在线更新前的负责人操作

1. 确认 Stable 与 Preview 的实际 HTTPS 静态索引地址、托管责任和访问范围；不得使用文档示例地址。
2. 在可信设备上按 [Tauri Updater 官方说明](https://v2.tauri.app/plugin/updater/)生成并备份 minisign 私钥；只把公钥提供给应用配置，私钥和口令不得进入仓库、聊天或日志。
3. 配置受保护的 GitHub Environment / Secrets 和审批规则。PR 与 Packaging Smoke 不得读取签名私钥。
4. 实现三平台 updater 归档和 `.sig`、Stable/Preview 独立索引生成、SemVer/平台/HTTPS/签名校验、索引防回退和发布后 HTTPS 读回；仅对已经公开且校验通过的 Release 发布索引。
5. 先完整安装首个带 updater 配置的版本，再用更高版本在 macOS ARM64、macOS Intel 和 Windows x64 实机验证跨渠道升级、签名拒绝、安装器接管及数据库/凭据留存。无法使用真实设备的项目标记 `BLOCKED`。
6. 降级需另行实现可审查的版本兼容策略与一致性备份，并完成 Windows MSI 和 macOS 替换安装矩阵。在这些条件通过前，保持当前拒绝降级的门禁。

不得生成假 `.sig`、占位 endpoint 或示例索引；不得关闭 TLS/签名检查、强制卸载或删除数据库来绕过降级失败。丢失 minisign 私钥会使现有客户端无法信任用新密钥签名的更新，轮换前必须设计兼容迁移。

## 验证与阶段门禁

本次执行结果、浏览器验证截图、分支和提交信息由交付记录说明。可在 Vite 视觉 Harness 中用 `/?harness=visual&page=settings-general&theme=light&locale=zh-CN&update=available` 查看设置；替换 `update` 参数查看上面的 Mock 场景。逐一验证中英文、明暗主题、窄窗口、渠道自动保存、相同/更高/更低版本、重复操作互斥和安装安全门禁。Harness 与构建通过不代表真实签名更新或跨版本安装通过。

| 阶段 | 状态 | 证据 / 未完成项 |
| --- | --- | --- |
| P0 基线与 API 对照 | PASS | 锁定 `tauri-plugin-updater 2.12.0`，按当前仓库实现并执行版本/API 检查。 |
| P1 版本策略与渠道持久化 | PASS | 严格 JS 版本规则、Rust 渠道/候选版本检查、可空设置和 MSI 版本映射测试。 |
| P2 渠道 UI 与 Native 控制器 | PASS WITH MOCK | UI、受控 Rust 命令和 Mock 已接入；当前无生产 endpoint / 公钥，真实服务仍未启用。 |
| P3 降级安全门禁 | PARTIAL / BLOCKED | 较低 SemVer 在下载前阻断，MSI overlay 显式允许降级；版本兼容策略、SQLite 一致性备份和 MSI/macOS 实机验证未完成。 |
| P4 签名打包与发布自动化 | BLOCKED | 等待负责人提供公钥并配置受保护签名 Environment / Secrets；本次未改发布工作流。 |
| P5 双渠道 HTTPS 索引 | BLOCKED | 等待负责人确认 Stable / Preview 实际托管地址；没有发布索引。 |
| P6 真实安装与数据回归 | BLOCKED | 需要较高版本的真实签名包，以及 macOS ARM64、Intel 和 Windows x64 设备。 |
| P7 收尾与交付 | IN PROGRESS | 本文和 `RELEASING.md` 已更新；Web 验收、推送和本地重建在本次交付记录中报告。 |

正式安装包、Release 工作流和操作系统代码签名状态见 [RELEASING.md](RELEASING.md)。
