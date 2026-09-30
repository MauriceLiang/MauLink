# Frontend v2 迁移基线：Phase 0

检查日期：2026-09-30（Asia/Shanghai）  
阶段状态：**BLOCKED，等待用户确认，不进入 Phase 1。**

## 基线与当前工作区

- 执行手册：`/Users/mauriceliang/Downloads/MauLink_AI前端架构改造执行手册.md`。
- 当前分支：`main`。
- HEAD：`ffe2e2440199b8a674096c5f5e55106fc8211348`（`前端优化`），与手册一致。
- 开始执行时已有 4 个未提交的前端文件改动，共 104 行新增、157 行删除。这些改动不是本次执行产生的，未覆盖、撤销或提交。
- 尚未建立迁移分支，尚未冻结包含这些改动的可复现提交。需要用户确认将它们纳入基线，还是以原 HEAD 为基线在独立 worktree 执行。
- 已通过 `git branch --show-current`、`git rev-parse HEAD`、`git status --short`、`git diff --stat` 和 `git diff --numstat` 核对。

| 既有修改文件 | 新增 / 删除 | 检查时 SHA-256 |
| --- | --- | --- |
| `frontend/index.html` | 2 / 3 | `6e78d084e1ce8b5c2ff4fc832a5d8a156426fe290a08cf1924f70bc94a3ed187` |
| `frontend/src/i18n.mjs` | 1 / 0 | `df01a578c84481b6914f67259a13382a311782293ecf3c147031cba9a5914229` |
| `frontend/src/main.mjs` | 66 / 103 | `7cb02b203266bedfff3d7834c6d1003156bc787330e6e806754529a58b2359f5` |
| `frontend/styles.css` | 35 / 51 | `6c371153cc7e39008d2cdc4ca792d7830aff0ac5932ba06f806218de55a63155` |

这些哈希只标识检查时的工作区内容，不能替代 Git 提交或完整备份。

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

本次检查针对包含既有未提交修改的工作区，不能视为仅对 HEAD 的检查结果。

| 命令 | 本次结果 |
| --- | --- |
| `cargo fmt --all -- --check` | PASS，退出码 0 |
| `node --check frontend/src/main.mjs` | PASS |
| `node --check frontend/src/i18n.mjs` | PASS |
| `node --check frontend/src/command-palette.mjs` | PASS |
| `node --check frontend/src/terminal-preferences.mjs` | PASS |
| `node --test frontend/tests/*.test.mjs` | PASS，24 passed / 0 failed |
| `git diff --check`（新增本报告之前） | PASS，退出码 0 |
| `cargo check --workspace --all-targets --locked` | 未执行，基线范围等待确认 |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | 未执行，基线范围等待确认 |
| `cargo test --workspace --locked` | 未执行，基线范围等待确认 |

环境版本：Cargo `1.98.1`、rustc `1.98.1`、Node.js `v26.10.0`。

`cargo tauri --version` 返回退出码 101：`error: no such command: tauri`。进一步检查发现已有临时 CLI：`/private/tmp/maulink-tauri-tools/bin/cargo-tauri --version` 成功返回 `tauri-cli 2.12.0`。因此不需要立即安装 CLI；恢复执行时可以使用 `PATH=/private/tmp/maulink-tauri-tools/bin:$PATH cargo tauri ...`，并重新确认临时路径可用。

## Desktop、功能与视觉基线

本次尚未启动或操作真实 Tauri Desktop；服务器列表、Add Server、Settings 的 Phase 0 运行验收均未完成。Light / Dark、中英文及 viewport 本次均未重新验证。历史文档中的结果不计为本次通过。

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

## Phase 0 验收报告与恢复入口

- 完成项：核对 HEAD、workspace、目录结构、Contract、命令边界及历史 QA；执行格式与旧前端检查；生成本报告。
- 本次修改文件：仅 `docs/refactor/frontend-v2-baseline.md`。
- 新增依赖：无。
- 与旧前端差异：无，本次未修改业务代码或正式配置。
- 当前阻塞：未提交修改应如何冻结为基线尚未确认；其余 Rust 检查和 Desktop 验收未完成。
- 退出条件：**BLOCKED**。报告存在，测试基线仍不完整，旧应用本次运行状态未验证。
- 推荐 commit：`docs(refactor): record frontend v2 migration baseline`，待基线确认及验收后使用。

恢复执行时先重新检查 branch / HEAD / 工作区。用户确认后建立 `refactor/frontend-v2`（手册指定），明确既有修改的基线归属，补齐 Rust 检查及真实 Desktop 验收，全部满足 Phase 0 退出条件后才能进入 Phase 1。

回滚参考提交是 `ffe2e2440199b8a674096c5f5e55106fc8211348`。该提交不包含既有未提交修改，不能通过直接 reset/checkout 覆盖这些文件来回滚；须先保全并明确其归属。本次没有创建 commit，也没有执行 reset、stash 或删除操作。
