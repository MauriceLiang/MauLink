# Phase 6 验收报告：Workspace 与 Terminal

检查日期：2026-09-30 至 2026-10-01（Asia/Shanghai）

状态：**BLOCKED**。前端自动化、Rust workspace 和既有真实 OpenSSH Terminal 集成测试通过；Browser 部分通过，但 Ctrl+C 实际键盘输入未获得 Core 接受确认，其他剩余操作尚未完成。用户于 2026-10-01 明确指示“直接推送进入”，允许先提交推送当前实现并进入 Phase 7；这是推进授权，不将未验证项改记为 PASS。

用户已明确取消后续 Tauri 手动验收门禁：自动化与 Browser 全部通过后可以按 PASS 处理并继续，Desktop GUI 未实测须如实记录。Phase 5 已按此规则提交并公开推送 `c4244f2d44a6ec05fa71b7b93a385c7f25bbfe51` 到 main。本次 Phase 6 提交前 HEAD 与 origin/main 均为此提交。

## 1. 根因、目标与完成项

Phase 5 的 SSH ready 尚未接入终端工作区。本阶段按手册读取 Core terminal.rs / connections.rs、Tauri terminal commands、旧 Terminal Channel / ACK、terminal-codec.mjs、terminal-preferences.mjs、旧 main.mjs Terminal 相关代码及 design-qa.md Page 7、8、14。

已实现：

- Workspace Header、标签、新建/关闭终端、状态栏、清屏、专注模式和终端设置。
- 正式链路保持 **Tauri Channel → 非响应式 Controller / xterm.write → write callback → terminal_ack**。Channel chunk、xterm、输出队列与渲染内容均不进入 Vue Store，Vue 仅保存标签和状态元数据。
- 输出按 decimal u64 序号顺序校验，校验 stream/terminal 身份、Base64 长度和 32 KiB chunk；待渲染队列保持 128 KiB 上限。pre-open 事件最多四块 / 128 KiB。协议错误暂停输入并显式关闭受影响 PTY，不影响其他标签。
- 输入串行发送，单块 64 KiB，最多暂存 256 KiB；Core 确认对应 inputSeq 后才递增。Unicode 用 UTF-8，onBinary 保留原始字节。错误后停止继续发送，不自动重放命令。
- ResizeObserver / requestAnimationFrame / FitAddon；80 ms debounce 与串行 terminal_resize，隐藏页面不提交零尺寸；重新显示只重新挂载现有 xterm。
- 关闭请求成功后才移除标签 / dispose renderer；失败保留会话与错误。组件 detach 不自动关闭 PTY。终态依据 terminal_get，停止轮询，避免重叠 poll 与重复结束提示。
- 专注模式保留退出入口；设置关闭和退出专注后恢复 xterm 焦点。全局 Cmd/Ctrl K 在 xterm 与 Dialog 内不截获。
- 终端设置复用 SettingsService / revision，修改字体、字号、光标与 scrollback，保留其他设置。选中即复制默认关闭，沿用 `maulink.terminal.copyOnSelect`，不新增凭据或 stdout 存储。
- DEV TerminalHarness 用临时回环 HTTP 桥接连接真实 Core / OpenSSH PTY。正式运行仍使用真实 Tauri Channel，不引入 HTTP 产品接口。

检查中修正：

1. 首轮 13 个新用例失败：测试环境 localStorage 不可用、mock 缺少 resize handler、设置测试错误地传入整份 defaultSettings。修正测试夹具和范围参数，保留全部行为验收。
2. 队列计数存在 ACK 竞态：Core 已处理 ACK 并发送下一窗口时，前端可能尚未收到 ACK 返回。已在 xterm callback 完成后释放已渲染字节计数，ACK 仍在 callback 后发送；新增延迟 ACK 返回的回归测试。该竞态由代码审查和测试确认，不能据此断言它是所有 Browser 突发输出停止的根因。
3. Vite 的 safari13 target 对 BigInt literal 发出兼容提示。改用十进制字符串精确递增，保留 u64 精度；测试覆盖超过 Number 安全整数的序号，不调整 WebView target。
4. 临时桥接离线依赖解析失败，改为复用仓库 Cargo.lock；桥接误 await 同步 terminal_get 已修正。仅改临时工程，仓库 Cargo 依赖、锁文件与 Core 未改。

当前键盘阻塞：Browser 的 `Control+c`、物理 KeyC 和原生 tab 键盘入口未得到 `terminal_write` 接受 ETX 的确认，DEV `Ctrl+C ACK` 保持 0，对应 Ctrl+C keydown 诊断也没有出现。已观察 top 显示/退出和普通输入，但不能把它们作为 Ctrl+C 中断已通过的证据。尚未区分 Browser 自动化组合键行为与终端实现原因。尝试 Codex 原生应用键盘通道时，Computer Use 工具明确因安全策略拒绝访问 `com.openai.codex`，没有绕过该限制。

## 2. 修改文件

```text
frontend-v2/package.json
frontend-v2/package-lock.json
frontend-v2/src/main.ts
frontend-v2/src/app/AppShell.vue
frontend-v2/src/app/LocalBackendStatus.vue
frontend-v2/src/terminal/codec.ts
frontend-v2/src/terminal/controller.ts
frontend-v2/src/terminal/preferences.ts
frontend-v2/src/components/terminal/XtermHost.vue
frontend-v2/src/components/terminal/TerminalTabs.vue
frontend-v2/src/components/terminal/TerminalWorkspace.vue
frontend-v2/src/dialogs/TerminalSettingsDialog.vue
frontend-v2/src/styles/terminal.css
frontend-v2/src/i18n/errors.ts
frontend-v2/src/harness/TerminalHarness.vue
frontend-v2/tests/terminal.test.ts
frontend-v2/README.md
docs/refactor/frontend-v2-phase-6.md
docs/refactor/screenshots/phase-6/browser-workspace-preview.jpg
```

本机临时验收辅助工程：`/private/tmp/maulink-phase6-bridge`，不在 Git 中。临时数据库、密钥和 sshd 来自现有 OpenSSH fixture；只核实生成公钥的指纹后 TrustOnce，不接触正式资料。

## 3. 新增依赖

| 名称 | 精确版本 | 用途与原因 |
| --- | --- | --- |
| @xterm/xterm | 5.5.0 | 终端 ANSI、字节流、输入及全屏程序；Vue / 现有 Base Components 不提供终端模拟器。与旧 vendor 版本一致。 |
| @xterm/addon-fit | 0.10.0 | 根据实际容器尺寸计算 PTY 行列；与旧 vendor 版本一致。 |

TypeScript 继续精确固定 `5.9.3`。Vue、vue-tsc 等其他依赖没有升级。lockfile 只增加这两个 xterm package。

## 4. 自动化检查

### Frontend

| 检查 | 结果 |
| --- | --- |
| npm run type-check | PASS，最终源码重新执行 |
| npm run test | PASS，7 files、66/66 tests；本阶段新增 15 |
| npm run build | PASS，91 modules；HTML 0.42 kB、CSS 21.26 kB、JS 447.19 kB（gzip 122.98 kB） |
| git diff --check | PASS |
| 正式 bundle 无 TerminalHarness / 127.0.0.1:1421 / Phase 6 fixture | PASS，构建产物文本搜索 |
| Core、Contracts、旧 frontend、Tauri 正式及 v2 配置无差异 | PASS，限定路径 git diff |

新增测试覆盖 write callback 后 ACK、序号/长度校验、128 KiB 队列上限、延迟 ACK 返回竞态、1 MiB 连续 stdout 不修改响应式 metadata、pre-open 事件、stream 身份失败、detach/remount 不关闭 PTY、Ctrl+C/Unicode/binary 字节、64 KiB 分片/256 KiB 输入边界、关闭失败保留/成功销毁、设置及默认复制偏好、非终端字段与 revision 保留、精确十进制序号。

Vitest 仍有原有 native config loader、jsdom Canvas 和 Node localStorage 非阻塞提示；Terminal Controller 测试使用可控 Renderer，Browser 使用真实 xterm。没有把 jsdom 测试当作实际 WebView 全屏/键盘验收。

### Rust

| 检查 | 结果 |
| --- | --- |
| fmt / check / clippy | 本阶段未重跑；未修改任何 Rust 文件，不宣称本轮 PASS |
| cargo test --workspace --locked | PASS，68 passed / 11 ignored（2026-09-30） |
| cargo test -p maulink-core --test openssh_terminal --locked -- --ignored --nocapture | PASS，1 passed；显式运行既有真实 OpenSSH Terminal 测试（2026-09-30） |

既有 Terminal 集成用例覆盖双 PTY 隔离、resize、输入序号/幂等、ACK 身份、128 KiB flow window、1,000,000 字节输出、消费者停滞/断开、close 与 connection disconnect。ignored 不自动算作通过；只将显式执行成功的 Terminal 用例记为 PASS。

## 5. Browser 实测

Codex 内置浏览器，真实 UI click / paste / key / select，临时桥接调用现有 Core，真实 loopback OpenSSH；不是静态生成 stdout。

| 操作 | 结果 |
| --- | --- |
| SSH ready → 首个真实 Shell | PASS |
| 中文 / ANSI 输出 | PASS，可见 `Phase6 中文 ANSI PASS` |
| 新建两个独立终端 / 标签切换 | PASS |
| 持续输出并隐藏原终端 / 操作新终端和设置 | PASS，80 × 256 × 58 = 1,187,840 字节 LF 文本（PTY 可能转 CRLF），约 8 秒分批生成，最终看到 `P6_BIG_DONE` 且 Shell 仍就绪 |
| 无延迟的突发 1 MiB 级输出 | 会话停止；未完成精确归因。现有 Core 有 512 KiB 总缓冲上限，仍须核实此次停止原因，不能标记此项 PASS |
| 字体 / 字号 / 光标 / scrollback 修改并保存 | PASS；尺寸由 137×33 变为 107×25，另一标签显示同步尺寸 |
| 设置关闭后的 xterm 焦点 | PASS，activeElement 为 Terminal input |
| 选中即复制默认关闭 | PASS，Dialog checkbox 初始未勾选；开启后真实系统剪贴板操作尚未验收 |
| 专注模式进入 / 退出 / 焦点恢复 | PASS，专注时 128×28，退出后回 xterm |
| top 显示 / q 退出回 Shell | PASS；不因此宣称 Ctrl+C 通过 |
| Ctrl+C 实际中断 | 未验证，Core ETX 接受计数保持 0，阻塞 |
| vim / nano 全屏编辑 | 未完成 |
| 快速切换 / 标签方向键 / viewport resize 对应 stty | 未完成完整验收；初始 stty 33×137 与 UI 一致，字体/专注 resize 已观察 |
| 关闭标签 / 断开 SSH / focus restore | Browser 未完成；相关自动化检查通过 |

[工作区预览](screenshots/phase-6/browser-workspace-preview.jpg) 仅说明界面已实现，不代表本阶段整体验收通过。

## 6. Desktop 实际验证与视觉验收

- Tauri Desktop：本阶段未启动实测。用户已取消该手动门禁，此项本身不阻塞，但不记为实际 PASS。
- Light / 中文 / 1440×920：已观察工作区、标签、设置 Dialog、焦点和字号变化；2026-10-01 保存最新预览。
- Dark / English / 最小 viewport：尚未完成本阶段完整视觉验收。
- 沿用旧黑色终端、灰色文本、紫色光标与既有 Shell token；对照旧 Workspace / Terminal 结构。未声称全部体验达到旧版。

## 7. 与旧前端差异、剩余风险

- 工程采用 Vue 管理 UI metadata，非响应式 Controller 管理 xterm 与 byte flow；正常页面切换通过 v-show 保留会话。
- 关闭成功后释放前端 runtime，失败不会伪造关闭；component unmount 不发 terminal_close。根 Shell HMR 或页面重新加载会丢失前端 runtime，开发验收须重置 fixture，不能将这种操作等同于正常 Tab 切换。
- u64 改为精确字符串递增，避免 BigInt literal 的旧 WebView build target 提示。
- 本机 HTTP 桥接增加额外延迟，只用于 Browser 功能验收，不足以证明 Tauri 原生 Channel 吞吐。突发输出停止尚需确认，未修改 Core bounded backpressure。
- 桥接源码暂在 /private/tmp，未随仓库保存，可恢复当前机器验收，但不是可移植的仓库集成工具。
- Keyboard、full-screen、close/disconnect、完整视觉验收尚有缺口；用户已明确允许保留缺口并进入 SFTP / Transfer。

## 8. 当前阻塞与恢复点

**BLOCKED：Browser Ctrl+C 未确认，无限速突发输出停止未精确归因，剩余 Browser 操作尚未完成。**

Phase 6 待补验收（不阻止本次用户授权的 Phase 7 推进）：

1. 区分 Browser 输入驱动与 xterm 键盘路径，获得实际 Ctrl+C → Core 接受 ETX → 中断运行命令 → 立即恢复 Shell 的证据。
2. 核实无限速大输出停止的 Core snapshot error / flow 原因，保留安全边界，不能只换成分批输出并宣称全部通过。
3. 完成 vim/nano、快速 Tab、键盘导航、resize、close/disconnect、复制偏好与完整视觉检查。
4. 必要代码修改后依次重跑 type-check、test、build，再完成 Browser 验收。
5. 补齐后更新实际验收状态。当前按用户新指示提交并公开推送 main，直接进入 Phase 7；后续阶段仍沿用自动化与 Browser 验收规则。

本报告随当前实现提交；实际提交与推送结果以 Git 记录为准。临时 Browser tab、Vite / HTTP bridge / fixture 已清理，不会再次要求用户手动检查 Tauri 界面。
