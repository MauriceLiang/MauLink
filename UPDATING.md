# MauLink 应用内更新

## 当前状态

本项目已实现 Stable / Preview 渠道偏好、版本规则、Rust 更新协调器和浏览器 Mock 流程；当前应用版本为 `0.1.1-rc.33`。正式更新源计划使用 GitHub Releases 保存安装与 updater 产物，使用独立 HTTPS 地址保存 `stable.json` / `preview.json`。仓库尚未启用更新索引托管，也没有 Updater 公钥或签名私钥，因此安装包仍不能在线更新；首次启用时必须手动安装一个带受信更新配置的完整应用包。

仓库已移除全部 GitHub Actions。合并后，PR、推送和 Tag 不会自动检查、构建、上传或发布；安装包由维护者在各平台手动编译并上传。手动上传 Release 不会部署更新索引，应用内更新仍保持未启用。正式安装包的手动流程见 [RELEASING.md](RELEASING.md)。

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

## GitHub 更新源

- 客户端构建时将 `MAULINK_UPDATER_INDEX_BASE_URL` 编译进 Tauri 配置，默认值为 `https://mauriceliang.github.io/MauLink/updates/`，渠道地址为 `stable.json` 与 `preview.json`。将来迁移到自有 HTTPS 主机时，可为新客户端改用新的索引基址；已安装客户端仍只信任其构建时写入的地址和公钥。
- GitHub Actions 发布工作流已移除。手动编译并上传普通安装包不会生成 Tauri updater 签名或更新索引；若未来启用应用内更新，必须另行人工生成并校验每个平台的签名更新包，再安全托管索引和下载文件。
- 索引生成按 SemVer 排序。较低版本不能覆盖较高版本；同一版本若下载地址或签名不同则失败。索引从实际 Release 资产读取签名内容，不使用占位签名。
- 每个平台包的 `.sig` 是 Tauri Updater 签名，不等同于 Apple Developer ID / 公证或 Windows Authenticode 签名。
- 当前 `0.1.1-rc.8` 没有编译更新公钥和地址，因此不能通过在线更新升级到首个启用 Updater 的版本。

### 未来启用在线更新前的发布准备

1. 先确定更新索引的 HTTPS 托管方式。当前没有 GitHub Actions 部署流程；如使用 GitHub Pages，需选用人工可维护的分支来源，或改用自有 HTTPS 主机。
2. 由维护者在可信设备生成并离线备份 Updater 密钥。私钥不得写入仓库、PR、Release 说明或聊天；公钥和索引基址须编译进新客户端。
3. 手动构建带受信更新配置的 Preview 客户端，为各平台生成 updater 包和 `.sig`，验证签名及下载后再手动上传 Release 和索引。普通 `.dmg` / `.msi` 安装包不能替代签名更新资产。
4. 先发布并手动安装新的 Preview 客户端，再用更高版本在真实设备验证更新。不要重用已有 Tag 或覆盖已发布附件。

如果后续把索引迁移到自有服务器，需更新客户端编译时的索引基址，并设计受保护的人工上传步骤。更新地址编译在客户端中；旧版本仍会访问原索引，因此迁移时需保留原索引作为兼容入口，或先发布并安装切换到新地址的过渡版本。

## 降级安全状态

SQLite 当前 schema 为 7，但项目尚无按发行版本记录的兼容性策略，且未完成 MSI 覆盖降级或 macOS `.app` 替换实机验证。因此 Rust 对所有 SemVer 降级一律拒绝，前端明确显示当前版本和目标版本。此阶段不提供降级二次确认或自动备份，因为尚无可验证的安全迁移规则；不能把 `allowDowngrades=true` 解释为降级已安全。直接使用完整 MSI 安装旧版仍需负责人先备份数据并按平台验收。

系统更新签名校验、HTTPS/TLS 校验没有关闭。应用 identifier、产品名、数据目录、SQLite 路径、凭据边界和 Windows MSI UpgradeCode 未变更。Tauri updater minisign 私钥与 Apple Developer ID、公证和 Windows Authenticode 证书相互独立。

## 启用在线更新前的负责人操作

1. 选定 HTTPS 索引托管方式，准备并保护 Updater 签名私钥；把公钥与更新地址编译进新客户端。
2. 维护者手动构建带更新配置的签名安装包和 updater 资产，逐一验证 `.sig`、下载地址和索引内容，再手动发布 Release 与 Stable / Preview 索引。
3. 先完整安装首个带 updater 配置的版本，再用更高版本在 macOS ARM64、macOS Intel 和 Windows x64 实机验证跨渠道升级、签名拒绝、安装器接管及数据库/凭据留存。无法使用真实设备的项目标记 `BLOCKED`。
4. 降级需另行实现可审查的版本兼容策略与一致性备份，并完成 Windows MSI 和 macOS 替换安装矩阵。在这些条件通过前，保持当前拒绝降级的门禁。

不得生成假 `.sig`、占位 endpoint 或示例索引；不得关闭 TLS/签名检查、强制卸载或删除数据库来绕过降级失败。丢失 minisign 私钥会使现有客户端无法信任用新密钥签名的更新，轮换前必须设计兼容迁移。

## 验证与阶段门禁

本次执行结果、浏览器验证截图、分支和提交信息由交付记录说明。可在 Vite 视觉 Harness 中用 `/?harness=visual&page=settings-general&theme=light&locale=zh-CN&update=available` 查看设置；替换 `update` 参数查看上面的 Mock 场景。逐一验证中英文、明暗主题、窄窗口、渠道自动保存、相同/更高/更低版本、重复操作互斥和安装安全门禁。Harness 与构建通过不代表真实签名更新或跨版本安装通过。

| 阶段 | 状态 | 证据 / 未完成项 |
| --- | --- | --- |
| P0 基线与 API 对照 | PASS | 锁定 `tauri-plugin-updater 2.12.0`，按当前仓库实现并执行版本/API 检查。 |
| P1 版本策略与渠道持久化 | PASS | 严格 JS 版本规则、Rust 渠道/候选版本检查、可空设置和 MSI 版本映射测试。 |
| P2 渠道 UI 与 Native 控制器 | PASS WITH MOCK | UI、受控 Rust 命令和 Mock 已接入；当前无生产 endpoint / 公钥，真实服务仍未启用。 |
| P3 降级安全门禁 | PARTIAL / BLOCKED | 较低 SemVer 在下载前阻断，MSI overlay 显式允许降级；版本兼容策略、SQLite 一致性备份和 MSI/macOS 实机验证未完成。 |
| P4 签名打包与发布流程 | BLOCKED | GitHub Actions 已移除；需要维护者确定密钥托管与人工签名构建流程，并在真实平台验收。 |
| P5 双渠道 HTTPS 索引 | BLOCKED | 尚无真实索引、签名包、人工部署流程或 HTTPS 读回证据。 |
| P6 真实安装与数据回归 | BLOCKED | 需要较高版本的真实签名包，以及 macOS ARM64、Intel 和 Windows x64 设备。 |
| P7 收尾与交付 | IN PROGRESS | 本文和 `RELEASING.md` 已更新；当前分支移除 GitHub Actions，版本提交仍需由维护者手动编译与发布。 |

正式安装包的手动构建、上传和操作系统代码签名状态见 [RELEASING.md](RELEASING.md)。
