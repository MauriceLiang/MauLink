# Frontend v2 迁移基线：Phase 0

检查日期：2026-09-30（Asia/Shanghai）  
阶段状态：**PASS：自动化检查通过，用户已反馈完成 Desktop 验收，允许进入 Phase 1。**

## 基线与当前工作区

- 执行手册：`/Users/mauriceliang/Downloads/MauLink_AI前端架构改造执行手册.md`。
- Phase 0 检查时分支：`refactor/frontend-v2`（已从干净的 `main` 创建）。
- 当前基线 HEAD：`b3da9ec6dadf63b2ff830b45873e68d6765c7f12`（`前端调试`）。用户已提交并推送前轮改动；本次通过 `git status --short` 确认开始时工作区干净。
- 手册原始 HEAD：`ffe2e2440199b8a674096c5f5e55106fc8211348`。用户提交解决了前轮基线归属问题，以下改动及本报告的前轮版本现已纳入 `b3da9ec`。
- 通过 `git branch --show-current`、`git rev-parse HEAD`、`git status --short` 和 `git show --stat --oneline HEAD` 核对。创建分支时沙箱首次禁止 `.git` 写入，自动审批后重试成功。

| 既有修改文件 | 新增 / 删除 | 检查时 SHA-256 |
| --- | --- | --- |
| `frontend/index.html` | 2 / 3 | `6e78d084e1ce8b5c2ff4fc832a5d8a156426fe290a08cf1924f70bc94a3ed187` |
| `frontend/src/i18n.mjs` | 1 / 0 | `df01a578c84481b6914f67259a13382a311782293ecf3c147031cba9a5914229` |
| `frontend/src/main.mjs` | 66 / 103 | `7cb02b203266bedfff3d7834c6d1003156bc787330e6e806754529a58b2359f5` |
| `frontend/styles.css` | 35 / 51 | `6c371153cc7e39008d2cdc4ca792d7830aff0ac5932ba06f806218de55a63155` |

表中哈希为前轮检查时的内容标识；当前可恢复基线以 `b3da9ec` 为准。

## 当前结构

```text
MauLink/
├── Cargo.toml / Cargo.lock / rust-toolchain.toml
├── contracts/v1/                  # 已存在，Rust DTO 导出的 TypeScript 类型
├── crates/maulink-core/
│   ├── src/
│   │   ├── app.rs / contracts/mod.rs / error.rs
│   │   ├── profiles.rs / credentials.rs / host_keys.rs / storage.rs
│   │   ├── connections.rs / ssh.rs / terminal.rs
│   │   ├── sftp.rs / sftp/transfer.rs / local_files.rs
│   │   └── monitor.rs / settings.rs
│   ├── migrations/
│   ├── examples/export_bindings.rs
│   └── tests/                     # Contract、OpenSSH、Monitor fixtures
├── frontend/
│   ├── index.html / styles.css
│   ├── src/
│   │   ├── main.mjs / i18n.mjs / icons.mjs
│   │   ├── command-palette.mjs / terminal-preferences.mjs
│   │   └── monitor-view.mjs / remote-path.mjs / terminal-codec.mjs
│   ├── tests/                     # 当前 24 项 Node 测试
│   └── vendor/                    # xterm、fit addon、Lucide 本地资源
├── src-tauri/
│   ├── src/main.rs / lib.rs / commands.rs / state.rs
│   ├── tauri.conf.json / tauri.harness.conf.json
│   └── build.rs / Cargo.toml / capabilities/main.json / icons/
├── tools/ipc-harness/
├── docs/
└── design-qa.md
```

Rust workspace members 为 `crates/maulink-core` 和 `src-tauri`，default-members 为 `crates/maulink-core`。桌面依赖固定 Tauri `2.11.6`；正式配置使用 `frontendDist = ../frontend`、`withGlobalTauri = true`。尚未创建 `frontend-v2/`。

## 自动化检查

用户明确授权修正 3 项 Clippy 问题后，本轮重新执行以下全部检查。所有命令退出码均为 0；没有降低 lint 门槛。

| 命令 | 本次结果 |
| --- | --- |
| `cargo fmt --all -- --check` | 本轮 PASS，退出码 0 |
| `node --check frontend/src/main.mjs` | 本轮 PASS，退出码 0 |
| `node --check frontend/src/i18n.mjs` | 本轮 PASS，退出码 0 |
| `node --check frontend/src/command-palette.mjs` | 本轮 PASS，退出码 0 |
| `node --check frontend/src/terminal-preferences.mjs` | 本轮 PASS，退出码 0 |
| `node --test frontend/tests/*.test.mjs` | 本轮 PASS，24 passed / 0 failed |
| `git diff --check` | 本轮 PASS，退出码 0 |
| `cargo check --workspace --all-targets --locked` | 本轮 PASS，退出码 0 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 修正后本轮 PASS，退出码 0；保留 `-D warnings` |
| `cargo test --workspace --locked` | 本轮 PASS，68 passed / 0 failed / 11 ignored |

环境版本：Cargo `1.98.1`、rustc `1.98.1`、Node.js `v26.10.0`。

`cargo tauri --version` 返回退出码 101：`error: no such command: tauri`。进一步检查发现已有临时 CLI：`/private/tmp/maulink-tauri-tools/bin/cargo-tauri --version` 成功返回 `tauri-cli 2.12.0`。因此不需要立即安装 CLI；恢复执行时可以使用 `PATH=/private/tmp/maulink-tauri-tools/bin:$PATH cargo tauri ...`，并重新确认临时路径可用。

## Desktop、功能与视觉基线

本轮隔离桌面 debug 构建通过：

```bash
PATH=/private/tmp/maulink-tauri-tools/bin:$PATH cargo tauri build --debug --bundles app --no-sign --config /private/tmp/maulink-migration-baseline.conf.json -- --locked
```

临时配置仅覆盖 `identifier=io.maulink.migration.baseline.20260930` 和 `productName=MauLink Migration Baseline`，未改正式配置；产物为 `target/debug/bundle/macos/MauLink Migration Baseline.app`（未签名）。该构建在收取 Clippy 结果时已启动，最终退出码 0。构建通过不代表启动或流程验收通过。

本轮尝试通过 CUA 打开上述隔离 `.app`，工具返回 `Computer Use was not approved to use MauLink Migration Baseline`，未能观察或操作实际窗口。工具没有提供更详细的拒绝原因。随后用户明确反馈“我已验证”，确认前轮交付清单中的启动、服务器列表、Add Server Dialog 和 Settings。此处运行验收证据来自用户反馈，不是 AI 自动操作或截图。Light / Dark、中英文及 viewport 本次均未重新验证。历史文档中的结果不计为本次通过。

代码及既有文档表明当前已实现：

- Server / Group CRUD，密码及私钥凭据处理。
- SSH 连接、Host Key 挑战、认证响应、取消及断开。
- Terminal PTY、多终端、输入、resize、Channel 输出和 ACK。
- SFTP 分页浏览、目录操作、真实上传下载及取消。
- Monitor snapshot / history / refresh 和 workspace/window 生命周期。
- Settings、主题、语言、终端偏好和 Command Palette。

这些是实现范围记录，不代表本次完整功能验收通过。

`design-qa.md` 仍为 `final result: blocked`，主要历史问题：

- 远程文本查看/编辑缺少后端 IPC，当前入口禁用，不应伪造正式能力。
- 截图未落盘、视口/DPR/数据状态不一致，无法严格视觉对照。
- 页面交互、深色回归、macOS 四种窗口尺寸及 Windows 真机覆盖不完整。
- Linux 成功监控指标没有可用的真实验证源。
- 旧 CSS 规则与后置覆盖尚未清理。

上述问题属于已有 QA 基线，后续按手册对应阶段处理；不能宣称已经解决。

## 迁移不可破坏的行为

- Core 保持 UI 无关，不重写 Rust 业务；Tauri 继续负责边界适配和资源清理。
- 保留 `contracts/v1/` 与 `apiVersion=1 + requestId + payload` 的字段语义；注意 `app_get_info` 当前是直接调用的例外，应以真实 command 签名为准。
- Terminal 保留 Channel、解析后 ACK 和有界背压，stdout 不进入 Vue reactive store。
- 凭据继续使用系统安全存储；禁止将密码或私钥口令持久化到普通前端存储。
- 未知 Host Key 明确询问，变化的 Host Key 默认拒绝。
- SFTP 保留 POSIX 路径、分页、Rust Transfer Manager 和 local file token 边界。
- Monitor 不支持或过期的指标不能冒充 0 或实时数据。
- 保留旧 `frontend/` 作为可运行和回归基线，正式切换须满足前置阶段门槛。

## 已授权修正与当前阻塞

用户已明确要求“执行修正”，完成以下无行为变化的最小调整：

| 文件 | 原 lint | 实际修正 |
| --- | --- | --- |
| `crates/maulink-core/src/monitor.rs` | `redundant_closure` | `.map(parse_finite_nonnegative)` |
| `crates/maulink-core/src/monitor.rs` | `type_complexity` | 添加模块内 `NetworkInterfaceRates` type alias，保留原元组返回语义 |
| `crates/maulink-core/src/sftp/transfer.rs` | `ptr_arg` | 内部 `local_temporary_path` 参数改为 `&Path`，原 `&PathBuf` 调用通过 coercion 保持兼容 |

workspace 检查验证调用方兼容，既有 Monitor 速率/解析测试和 SFTP 单元测试通过。未改变 DTO、IPC、计算逻辑、路径构造或错误语义，未添加依赖，也未添加仅重复实现的测试。

Rust 测试统计：Core 64 passed / 1 ignored，Contract 4 passed；OpenSSH Connection 2 ignored、Lifecycle 1 ignored、SFTP/Monitor/大文件矩阵 6 ignored、Terminal 1 ignored，总计 68 passed / 11 ignored。ignored 项仍需各自 fixture 或原生服务验证，不能算作本轮通过。

Desktop 控制未获准的问题通过用户手动验收解决。用户确认后续凡需应用操作，AI 应停止并交付具体步骤，等待用户操作反馈。后续阶段 Desktop 验收仍执行该门禁，不能因 Phase 0 通过而自动视为通过。

## Phase 0 验收报告与恢复入口

- 完成项：确认新基线与迁移分支；按用户授权修复 3 项 Clippy；全部 Rust 自动检查、旧前端语法检查及 24 项测试通过。
- 本次修改文件：`crates/maulink-core/src/monitor.rs`、`crates/maulink-core/src/sftp/transfer.rs`、`docs/refactor/frontend-v2-baseline.md`。
- 新增依赖：无；复用临时路径中的 Tauri CLI `2.12.0`。
- Desktop：隔离 debug bundle 构建通过；AI 自动控制未获准；用户随后确认启动及目标页面操作通过。
- 视觉：Light / Dark、中文 / English、viewport 均未重新验收；保留原 QA 限制。
- 与旧前端差异：无；无业务行为或正式配置修改，仅等价 Rust 代码修正。
- 当前 Phase 0 阻塞：无。历史 QA 限制保持记录，后续按阶段处理。
- 退出条件：**PASS**。基线文档存在，自动检查通过，用户反馈旧应用运行及目标页面操作通过。
- 推荐 commit：代码修正为 `fix(core): resolve monitor and sftp clippy warnings`；基线记录为 `docs(refactor): record frontend v2 migration baseline`。该阶段修正与报告按用户后续授权，在 Phase 2 验收通过后提交到主分支；提交范围见 Phase 2 报告。

根据用户反馈，Phase 0 已通过，开始 Phase 1。Phase 1 的新前端 Desktop 加载及 HMR 需另行交付用户验收。

回滚基线为 `b3da9ec6dadf63b2ff830b45873e68d6765c7f12`，包含用户提交的前端调整和本报告前轮版本。没有执行 reset、stash 或删除操作。
