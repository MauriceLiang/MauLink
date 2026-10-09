# MauLink 弹窗与按钮统一风格改造交付报告

- 实施日期：2026-10-09（Asia/Shanghai）
- 基准提交：`cb016fad09d600ba366c0b7167eda64ebc80c41d`
- 分支：`feature/add-modal-button-unification`
- PR：由用户创建；本次不创建 PR。
- 执行范围：实施手册 P0–P10。改造仅涉及前端、视觉夹具、测试和截图说明；Rust、SSH/SFTP、安全协议及数据库未改动。

## 阶段记录

| 阶段 | 结果 | 实施内容与验收 |
| --- | --- | --- |
| P0 | 完成 | 确认基准提交和干净 worktree，保留弹窗/设置关键场景的改造前截图。 |
| P1 | 完成 | 建立 `primary`、`secondary`、`softPrimary`、`danger`、`ghost`、`icon` 六类按钮及 `sm`、`md`、`lg` 尺寸。 |
| P2 | 完成 | 统一 Dialog 尺寸语义，保留 AlertDialog 的危险确认与焦点行为。 |
| P3 | 完成 | 收紧删除服务器、删除分组、断开 SSH 确认框；保留活动传输停止确认。 |
| P4 | 完成 | 统一顶部添加、侧栏添加和 Quick Monitor 入口的按钮权重与可用状态。 |
| P5 | 完成 | 服务器卡片主体可导航，更多操作保持独立，不嵌套按钮。 |
| P6 | 完成 | 设置迁入主 Shell 页面；保留终端工作区挂载，普通设置草稿/保存/放弃与安全、GeoIP 独立事务。 |
| P7 | 完成 | 回归连接、凭据查看、文件编辑、命令面板、终端断开等弹层调用。 |
| P8 | 完成 | 更新组件、设置、服务器卡片、断开传输和视觉场景测试。 |
| P9 | 完成 | 采集并目视检查 60 张改造后截图：35 张中文浅色、12 张中文深色核心、6 张英文 860×640、5 张自定义红色紧凑布局、浅/深按钮总览各 1 张。重复刷新逐张一致，DPR=1，均无横向溢出。 |
| P10 | 完成 | 差异检查、Rust 与前端验证、截图校验和 macOS `.app` 重建均完成。原 MauLink Blue 82 张参考基线未覆盖；Windows 原生验收未验证。 |

## 视觉证据

改造前后图片、条件 manifest 和 SHA-256 记录：[`screenshots/modal-button-v1-evidence/`](screenshots/modal-button-v1-evidence/)。已确认的 MauLink Blue 基线仍保留在 `screenshots/color-v1-review/`。

| 场景 | 改造前 | 改造后 |
| --- | --- | --- |
| 服务器列表 | [浅色](screenshots/modal-button-v1-evidence/before/servers-light-zh-CN-1440x920.jpg) · [深色](screenshots/modal-button-v1-evidence/before/servers-dark-zh-CN-1440x920.jpg) | [浅色](screenshots/modal-button-v1-evidence/after/light-zh-CN-1440x920/servers-light-zh-CN-1440x920.jpg) · [深色](screenshots/modal-button-v1-evidence/after/dark-zh-CN-1440x920-core/servers-dark-zh-CN-1440x920.jpg) |
| 编辑服务器 | [浅色](screenshots/modal-button-v1-evidence/before/edit-server-light-zh-CN-1440x920.jpg) · [深色](screenshots/modal-button-v1-evidence/before/edit-server-dark-zh-CN-1440x920.jpg) | [浅色](screenshots/modal-button-v1-evidence/after/light-zh-CN-1440x920/edit-server-light-zh-CN-1440x920.jpg) |
| 删除分组 | [浅色](screenshots/modal-button-v1-evidence/before/delete-group-light-zh-CN-1440x920.jpg) · [深色](screenshots/modal-button-v1-evidence/before/delete-group-dark-zh-CN-1440x920.jpg) | [浅色](screenshots/modal-button-v1-evidence/after/light-zh-CN-1440x920/delete-group-light-zh-CN-1440x920.jpg) · [深色](screenshots/modal-button-v1-evidence/after/dark-zh-CN-1440x920-core/delete-group-dark-zh-CN-1440x920.jpg) |
| 有活动传输时断开 | [浅色](screenshots/modal-button-v1-evidence/before/disconnect-with-transfer-light-zh-CN-1440x920.jpg) · [深色](screenshots/modal-button-v1-evidence/before/disconnect-with-transfer-dark-zh-CN-1440x920.jpg) | [浅色](screenshots/modal-button-v1-evidence/after/light-zh-CN-1440x920/disconnect-transfer-light-zh-CN-1440x920.jpg) · [深色](screenshots/modal-button-v1-evidence/after/dark-zh-CN-1440x920-core/disconnect-transfer-dark-zh-CN-1440x920.jpg) |
| 设置页面 | [通用浅色](screenshots/modal-button-v1-evidence/before/settings-general-light-zh-CN-1440x920.jpg) | [通用](screenshots/modal-button-v1-evidence/after/light-zh-CN-1440x920/settings-general-light-zh-CN-1440x920.jpg) · [安全与隐私](screenshots/modal-button-v1-evidence/after/light-zh-CN-1440x920/settings-security-light-zh-CN-1440x920.jpg) |
| 按钮总览 | — | [浅色](screenshots/modal-button-v1-evidence/after/button-overview/button-overview-light.jpg) · [深色](screenshots/modal-button-v1-evidence/after/button-overview/button-overview-dark.jpg) |

## 验证结果

- `git diff --check`：通过。
- `cargo fmt --all -- --check`：通过。
- `cargo clippy --workspace --all-targets --locked -- -D warnings`：通过。
- `cargo test --workspace --locked`：通过，113 项通过、11 项忽略。忽略项需要原生凭据库或隔离 OpenSSH 服务。
- `cargo check --workspace --all-targets --locked`：通过。
- `npm --prefix frontend run type-check`：通过。
- `npm --prefix frontend run test`：通过，26 个文件、239 项测试。
- `npm --prefix frontend run build`：通过；现有主 JS chunk 为 943.30 kB，仍有 Vite 500 kB 提示，本次未做无关拆包。
- `node --test frontend/visual/capture.test.mjs`：通过。
- `node frontend/visual/verify.mjs`：通过，已确认的 82 张颜色基线哈希和尺寸有效。
- Web 视觉验证：60 张改造后截图均通过两次独立 reload、主题/语言/密度/DPR/字体和无溢出检查；新增安全与隐私页面和活动传输断开流程已在实际页面确认。
- macOS 构建：`cargo tauri build --bundles app --ci -- --locked` 完成，生成 `target/release/bundle/macos/MauLink.app`（30.07 MiB），未见 bundle 损坏警告。Windows 原生构建及手工 UI 验收未验证；Browser 验收不等于 Windows/macOS 原生运行验收。

## 主要改动位置

按钮与弹窗基元在 `frontend/src/components/base/`；设置页面位于 `frontend/src/app/settings/` 并由 `frontend/src/app/AppShell.vue` 接入；服务器、连接、终端和监控入口迁移位于 `frontend/src/components/`、`frontend/src/dialogs/` 与 `frontend/src/styles/`。相关回归覆盖 `frontend/tests/`；页面步骤与截图脚本位于 `frontend/visual/`。截图场景夹具补上外观读取响应，编辑服务器截图不再显示夹具错误。

## 已知提示与未验证项

- Vite 生产构建仍报告主 JS chunk 超过 500 kB；构建成功，该提示与本次视觉改造无关。
- 若系统需要单独安装原生凭据库或启动隔离 OpenSSH，相关集成测试在默认工作区测试中按仓库设置忽略。
- Windows WebView2、Windows 安装包及实际原生系统文件选择器未在此 macOS 环境中验证。
