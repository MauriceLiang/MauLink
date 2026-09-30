# Phase 5 验收报告：SSH Connection 与安全交互

检查日期：2026-09-30（Asia/Shanghai）

状态：**PASS（按用户更新后的门禁）：自动化、Browser 与 Core SSH fixture 检查已通过。用户明确取消本轮及后续 Tauri 手动验收要求，允许按 Browser 通过处理并继续。Tauri GUI 未实测，不记为实测 PASS；可提交推送并进入 Phase 6。**

Phase 4 已全部 PASS，提交并公开推送 `dcabd92e7b4cec79b43d442983fa42be261003db` 至 main，提交后 HEAD / origin/main 一致、工作区干净。用户授权后续通过阶段继续自动公开推送。

## 1. 根因与目标

Phase 4 仅展示服务器资料，尚未迁移连接入口和安全挑战。读取手册 Phase 5、六个既有 Tauri commands、对应 DTO、Core connections.rs / ssh.rs / host_keys.rs、旧连接轮询与安全弹窗，以及 design-qa.md Page 6。目标为通过现有 Core 建立真实 SSH 会话，保持安全默认值。

运行态发现安全弹窗关闭时原连接 trigger 已被状态切换移除，焦点落至 body。已恢复至当前可用操作，并增加回归测试、Browser 重跑。两次测试驱动预期修正分别为：认证完成后弹窗已经消失，不能继续查找输入；弹窗关闭早于下一次 Core snapshot，焦点验收须等待最终状态。保留替代行为断言与等待期间清空输入测试，没有删除验收。

临时 SSH fixture 首轮错误密码检查 FAIL：默认 Auth::Reject 在拒绝密码后不再声明 Password 为可用方式，因此 Core 返回 AuthMethodUnsupported。修正 fixture 的拒绝响应，明确继续支持 Password/PublicKey 后，原测试 PASS；未改 Core 或弱化错误判断。

## 2. 修改内容

- 新增 connections Store：仅保存 Core ConnectionSnapshot、请求 busy 与结构化错误；以既有 connection_get 轮询，不自行推演 SSH 状态。终态停止轮询，卸载释放定时器。
- start 使用 saved serverId / expectedRevision 与 workspace mode；同一服务器重复启动被阻止。cancel / disconnect 调用真实接口；disconnect 不隐式取消传输任务。
- 通过请求版本避免取消或挑战响应后的迟到轮询覆盖新结果。轮询故障保留最后 Core snapshot，提供刷新和取消，不伪造失败/断开。
- 首次 Host Key 显示实际 host、port、algorithm、SHA256 指纹；默认焦点为拒绝入口，Esc/关闭即 reject。不会自动回应挑战。
- Changed Host Key 展示旧、新指纹及更高风险提示；不显示 trustOnce，更新记录须显式展开并勾选独立核实。没有自动接受未来变化的选项。
- AuthenticationChallenge 沿用 challengeId / connectionId 与 auth_respond；密码或私钥口令遮罩显示，提交前立即清空输入，仅用于本次连接，不保存在 Store、浏览器存储或日志。
- busy 弹窗禁用操作、Esc 不关闭；认证 Esc 真正 cancel。挑战响应成功后抑制旧 challenge 再次弹出，仍由 Core snapshot 决定后续状态。
- 安全错误文案、阶段、折叠诊断；诊断只显示 code / stage / requestId，不直接渲染 Rust debug details。Retry 仅在终态且 error.retryable 时显示，HOST_KEY_CHANGED 始终不提供 Retry。非重试错误可显式关闭后开始新尝试。
- ConnectionPanel 和 Home 卡片展示 snapshot 状态；建立 SSH 后明确告知终端工作区留在 Phase 6，无虚假终端 UI。
- 开发专用 Mock / Native SSH Harness，含 10 秒延迟挑战响应；正式构建排除 Harness。

## 3. 修改文件与依赖

```text
frontend-v2/src/stores/connections.ts
frontend-v2/src/components/connection/ConnectionPanel.vue
frontend-v2/src/components/connection/ConnectionError.vue
frontend-v2/src/dialogs/ConnectionDialogs.vue
frontend-v2/src/i18n/connections.ts
frontend-v2/src/i18n/errors.ts
frontend-v2/src/app/AppShell.vue
frontend-v2/src/components/server/ServerCard.vue
frontend-v2/src/components/server/ServerList.vue
frontend-v2/src/harness/ConnectionHarness.vue
frontend-v2/src/harness/connection-fixtures.ts
frontend-v2/src/styles/connections.css
frontend-v2/src/main.ts
frontend-v2/tests/connections.test.ts
frontend-v2/README.md
docs/refactor/frontend-v2-phase-5.md
docs/refactor/screenshots/phase-5/*.jpg
```

无生产/前端依赖变化。TypeScript 仍精确 5.9.3。Core、Contracts、旧 frontend、正式与 v2 Tauri 配置均未改。

## 4. 自动化检查

| 检查 | 结果 |
| --- | --- |
| npm run type-check | PASS |
| npm run test | PASS，6 files、51/51 tests（本阶段新增 13） |
| npm run build | PASS，74 modules；HTML 0.42 kB、CSS 15.85 kB、JS 133.52 kB |
| cargo test --workspace --locked | PASS，68 passed / 11 ignored；不把 ignored 算作通过 |
| 既有 SSH connection 集成测试显式 --ignored --exact | PASS，1 passed / 1 filtered out，使用临时回环 russh fixture |
| git diff --check | PASS |
| Core / Tauri / 旧 frontend / Contract / package 与 lockfile 无差异 | PASS |
| 新业务模块无散落 invoke / localStorage / sessionStorage | PASS |
| 正式 bundle 排除 Mock SSH、fixture 指纹和延迟挑战控件 | PASS |

新增用例覆盖 revision/workspace、重复启动、轮询释放、迟到 poll / cancel、无自动信任、指纹信息与 Esc 拒绝、变化指纹显式核实、busy Esc、凭据即时清空、错误密码、口令挑战取消、disconnect 不强制停传输、Retry 约束、折叠安全诊断、轮询故障与真正取消、响应过期错误保留、trigger 消失后的焦点。

显式运行仓库 `connects_to_isolated_openssh_and_checks_authentication_errors`，设置现有 MAULINK_OPENSSH_FIXTURE_* 和错误密码/加密私钥 flags。实际服务为独立临时 russh 0.63.3，故只能证明真实 SSH 协议与 Core 路径，不宣称本次执行了完整 OpenSSH 服务矩阵。覆盖未加密/加密私钥、错误密码/口令、信任/拒绝/变化、取消和拒绝连接。正常密码成功的 Native GUI 路径未实测；用户已取消该项作为阶段阻塞门禁。

Vitest 提示未来 native config loader 导入扩展名为非阻塞提示，未扩大配置修改。

## 5. Browser 实测：PASS

Codex 内置浏览器，通过真实 click/fill/select/press 操作内存 Mock：

- 首次 Host Key 信息完整，初始焦点拒绝；Tab 从 lastFocusable 回 firstFocusable，Shift+Tab 反向循环。
- 仅本次信任 → 密码挑战 → ready → disconnect → closed；返回首页卡片显示已连接状态通过。
- Changed Host Key 旧/新指纹、危险提示、默认收起更新入口；未勾选核实时按钮 disabled，明确核实后更新才进入认证。
- Changed Esc 拒绝、错误密码可读文案、不展示 Retry；焦点回当前可用操作。
- Timeout 只在 retryable 时显示重试；重新启动产生新连接；代理失败有具体安全文案和阶段。
- 私钥口令挑战 Esc 取消、回 cancelled；输入仅用于一次认证。
- 延迟挑战回应 10 秒：真实 Escape 后 dialog 仍打开、aria-busy=true、全部按钮 disabled；完成后才进入新认证挑战。
- 1440×920 Light/Dark；860×640 Changed Dialog 位于 x167/y74.05、526×491.88、bottom565.94，scrollWidth860，无横向溢出，底部操作与键盘循环正常。
- 最终 Browser console warning/error 为空；临时标签关闭、viewport reset、独立 Browser Vite 已停止。

Mock 不作为真实网络、钥匙串或 WebView 行为证明。

截图（1440×920，带 860 的为 860×640，已由 sips 核对）：

- [首次 Host Key Light](screenshots/phase-5/host-key-light.jpg)
- [Changed Dark](screenshots/phase-5/host-key-changed-dark.jpg) / [最小尺寸](screenshots/phase-5/host-key-changed-dark-860.jpg)
- [认证 Dark](screenshots/phase-5/authentication-dark.jpg)
- [Busy Light](screenshots/phase-5/host-key-busy-light.jpg)
- [已连接 Light](screenshots/phase-5/connected-light.jpg)
- [超时诊断 Dark](screenshots/phase-5/timeout-diagnostics-dark.jpg)

## 6. Tauri 实测：未执行（用户取消该门禁）

Native Harness 使用真实 IPC 和 Core，临时 overlay 不修改正式配置：

```bash
PATH=/private/tmp/maulink-tauri-tools/bin:$PATH cargo tauri dev --config src-tauri/tauri.frontend-v2.conf.json --config /private/tmp/maulink-phase5-native.json -- --locked
```

devUrl：`?harness=connections&transport=native&theme=light`。必须看到“Native SSH 验收”，不能将 Mock 结果计作实机通过。Vite ready，Rust dev 编译 PASS（8.08s），target/debug/maulink 已运行。窗口启动/编译结果只证明运行入口，GUI 由用户操作。

一次性 fixtures 均在 /private/tmp，不在仓库：

- SSH：`127.0.0.1:42525`，用户 `phase5`、一次性密码 `phase5-fixture`；仅接受测试密码和指定测试公钥，没有 shell/文件/转发权限，10 分钟空闲限制。
- 私钥：`/private/tmp/maulink-phase5-fixture/client_ed25519`（无口令）及 `client_encrypted_ed25519`（口令 `fixture-passphrase`）。
- 无 SSH banner 的 TCP：`127.0.0.1:42424`，只用于维持 connecting 并验证 cancel / timeout。复用上阶段临时脚本，启动日志仍标 Phase 4，不影响 fixture 行为。
- 拒绝连接：`127.0.0.1:42526`，当前无服务。

原定操作清单（保留供后续回归参考，本轮不再请求用户执行）：

1. 新增 `P5-Password`，127.0.0.1:42525 / phase5 / 密码认证，保存时密码留空。查看 → 连接，首次弹窗核对 host/port/algorithm/fingerprint；Esc 应拒绝。关闭错误再连接，选择“仅本次信任”，认证输入一次性密码，应显示已连接；返回首页状态一致；断开应显示已断开。
2. 新增 `P5-Key`，相同地址和用户，SSH 密钥，通过 picker 选择 client_encrypted_ed25519，保存口令留空。再次 Host Key 说明仅本次信任未持久化；可明确“信任并保存”，输入 fixture-passphrase 后应已连接，随后断开。不要求提供任何真实账户凭据。
3. P5-Password 再连接，输入 wrong 应显示可读认证失败，无自动 Retry；关闭错误后重新连接，在认证弹窗 Esc 应实际取消。
4. 新增 `P5-Cancel`，127.0.0.1:42424 / phase5 / 密码留空，高级 timeout=10000。连接中点击取消应变已取消；再连接等待应显示超时与合理 Retry。
5. 新增 `P5-Unreachable`，127.0.0.1:42526 / phase5 / 密码留空。连接应显示拒绝连接，诊断默认折叠，可展开 code/stage/requestId，不泄露原始 details。
6. Light/Dark、最小尺寸、Tab/Shift+Tab、Esc、焦点恢复、拖动窗口正常；没有自动信任。Changed 场景由 Browser fixture + 自动化/Core 测试覆盖，本轮不修改真实 known_hosts 制造变化。
7. 断开全部验收连接；可删除本轮 P5-* 一次性配置。临时 SSH / TCP / Tauri 进程在用户取消手动验收后停止。

## 7. 剩余风险与退出条件

当前无阻塞。用户最新指令优先于手册原人工门禁：后续以自动化检查与 Browser 实测通过为阶段 PASS 条件，不要求继续手动操作 Tauri。真实 WebView、正常密码/私钥 GUI 成功、取消/断开仍属未实测范围，不以 Browser Mock 替代其证据。

非阻塞范围限制：Windows 未运行；完整 English/Settings 后续迁移；本阶段不接入终端/SFTP/Monitor；其余 ignored 集成测试未运行。临时 russh fixture 不代表所有 OpenSSH/生产服务器兼容性；没有请求真实密码/私钥材料。JS 字符串无法承诺物理内存 zeroization，仅清除 UI 引用并限制生命周期。

本阶段 commit：`feat(frontend-v2): migrate secure ssh connection flow`。按用户持续授权写完整提交正文并公开推送 main；本报告随阶段代码提交；实际 SHA 与推送状态以 Git 记录为准。
