# Phase 2 验收报告

检查日期：2026-09-30（Asia/Shanghai）

状态：**PASS：type-check、17/17 测试、build、Browser 实际键盘操作和用户 Tauri WebView 验收全部通过。本轮停在 Phase 2，不进入 Phase 3。**

## 1. 完成项

- Phase 1 全部通过后，按用户最初的顺序执行授权进入 Phase 2。
- 核对 Rust Contract、43 个带 envelope 的真实 command 签名，以及 `app_get_info` 的例外签名。
- 建立 Typed IPC client 与 6 个领域 facade；输入/输出类型直接引用 `contracts/v1/`，不手工复制 DTO。
- 普通请求统一创建 `apiVersion=1`、独立 UUID requestId 和 payload。
- Terminal open 与 SFTP upload/download 继续要求独立的 typed Channel 参数；没有迁移高频流、修改 ACK 或加入 reactive stdout。
- 添加可注入 Mock IPC、模块化错误 catalog、AppError mapper/presenter。
- 添加 7 个 Base Component 与集中样式、开发模式的基础组件 Browser Harness。
- 添加 Vitest / Vue Test Utils / jsdom 测试，TypeScript 保持精确 `5.9.3`。
- 本轮仅修正键盘焦点问题，保持 BaseButton 原有 native disabled 实现，不增加 blur()，不改动 IPC、Error Mapper 或其他业务基础。
- 按用户最新授权，在 Codex 内置浏览器完成真实键盘操作验收；Browser 通过后已启动 Tauri Harness，应用界面由用户执行验收，用户反馈“通过”。

### 本轮根因与修改内容

1. **BaseButton 测试假设错误**：已聚焦按钮动态 disabled 后，原测试调用 blur()/focus() 并要求 activeElement 必须变化。已检查本地 jsdom 实现：blur() 对不再可聚焦的 disabled 元素直接返回；该断言依赖模拟 DOM 的动态失焦细节，不能验证用户 Tab 导航。替换为 enabled、初始 disabled、聚焦后动态 disabled/loading 三组测试：原生 disabled 属性、aria-busy、native click 不触发 action、重新 enabled 可操作。真实 Tab 排除与视觉状态留给真实 Browser/WebView 验收，不删除验收。
2. **Dialog 候选顺序错误**：完整 DOM 为关闭按钮 → Input → 末尾按钮，本地 jsdom 的复杂 selector-list 查询却返回关闭按钮 → 末尾按钮 → Input。改用单一 `*` 查询按 DOM 顺序收集并筛选；正 tabindex 优先，其余保留 DOM 顺序。统一排除 disabled（包含显式 tabindex=0 的 disabled 元素）、负 tabindex、hidden input 和 hidden/inert/aria-hidden 子树。初始焦点与 Tab/Shift+Tab 使用同一候选序列。
3. **回归验收**：测试以 firstFocusable/lastFocusable 标识边界，保留 Input 后接按钮的原始失败场景，并新增末尾 Input、其后的 disabled 元素、正 tabindex、busy 初始焦点/双向循环/Esc 和关闭后 trigger 焦点恢复。
4. **Harness**：添加动作计数、动态禁用开关、busy 开关；对话框末尾可聚焦元素明确为 Input，其后保留 disabled tabindex=0 按钮。busy 仍禁止 Esc 和关闭按钮，取消 busy 恢复既定关闭规则。

## 2. 修改文件

本轮修改：

- `frontend-v2/src/components/base/BaseDialog.vue`
- `frontend-v2/tests/components.test.ts`
- `frontend-v2/src/harness/FoundationsHarness.vue`
- `docs/refactor/frontend-v2-phase-2.md`

BaseButton.vue 已检查，无需修改；本轮没有新增依赖或更新 lockfile。以下为整个 Phase 2 的文件范围。

新增目录/文件：

```text
frontend-v2/src/ipc/
  commands.ts / client.ts / mock.ts
  server.ts / connection.ts / terminal.ts / sftp.ts / monitor.ts / settings.ts
frontend-v2/src/errors/mapper.ts
frontend-v2/src/errors/presenter.ts
frontend-v2/src/i18n/errors.ts
frontend-v2/src/components/base/
  BaseButton.vue / BaseIconButton.vue / BaseInput.vue / BaseDialog.vue
  BaseToast.vue / BaseEmptyState.vue / BaseStatusBadge.vue
frontend-v2/src/styles/components.css
frontend-v2/src/harness/fixtures.ts
frontend-v2/src/harness/FoundationsHarness.vue
frontend-v2/tests/ipc.test.ts
frontend-v2/tests/errors.test.ts
frontend-v2/tests/components.test.ts
frontend-v2/vitest.config.ts
docs/refactor/frontend-v2-phase-2.md
```

更新：`frontend-v2/package.json`、`package-lock.json`、`tsconfig.json`、`src/main.ts`、`src/styles/themes.css`、`README.md`。Phase 1 报告更新为 PASS。此前获授权的 Core Clippy 修改仍在工作区，本阶段没有追加 Rust/Core/Contract/Tauri command 修改。

## 3. 新增依赖

| 名称 | 精确版本 | 用途与理由 |
| --- | --- | --- |
| `@tauri-apps/api` | `2.11.0` | 模块化 invoke / Channel 类型，与 Rust Tauri 2.11 对齐；现有空壳没有该模块 API |
| `vitest` | `4.1.11` | 新前端单元/组件测试；兼容既有 Vite 8 与 Node 最低版本 |
| `@vue/test-utils` | `2.5.1` | Vue SFC 挂载及交互测试 |
| `jsdom` | `27.4.0` | 测试 DOM、焦点和键盘逻辑；不等同于真实 WebView |

Vue `3.5.43`、vue-tsc `3.3.11`、TypeScript `5.9.3`、Vite `8.3.1`、Vue 插件 `6.0.9`、Node 类型 `26.6.3` 保持不变，没有追逐 TypeScript 7 或引入 UI 框架、Pinia。只有 Tauri API 为新增 runtime 依赖；测试包仅开发使用。官方 registry 的 engine/peer 范围已核对。安装成功，npm audit 为 140 packages、0 vulnerabilities；可选 fsevents install script 提示仍未自行批准。

## 4. 自动化检查

### Frontend

本轮依次执行（2026-09-30）：

- `npm run type-check`：**PASS**，退出码 0，TypeScript 精确固定 5.9.3。
- `npm run test`：**PASS**，退出码 0；3 个文件、17/17 项全部通过（原为 11 passed / 2 failed）。
- `npm run build`：**PASS**，退出码 0；Vite 8.3.1，17 modules，成功生成 dist。
- `npm run dev`：**启动 PASS**，Vite ready，127.0.0.1:1420；Harness 路径 `/?harness=foundations`。启动/HTTP 成功不等于实际键盘验收。
- `git diff --check`：**PASS**。
- `git diff --exit-code -- frontend src-tauri/tauri.conf.json contracts/v1 src-tauri/src/commands.rs`：**PASS**。旧前端、正式配置、IPC Contract 与 commands 无差异。本轮没有追加 Rust 修改。
- Vitest 保留非阻塞提示：未来 native config loader 不支持 extensionless `./vite.config` 导入。本轮范围仅键盘问题，未额外改动配置。

### Rust

本阶段没有追加 Rust 修改。Phase 0 的 fmt / check / clippy / test 保持历史记录。本轮 Tauri `cargo run --locked --no-default-features` dev 编译 PASS（11.62s），已运行 target/debug/maulink；不将 dev 编译冒充完整 Rust 测试。

## 5. Browser / Desktop 实际验证

### Browser Harness

- 是否启动：是，Vite dev server 已启动。
- 页面：`http://127.0.0.1:1420/?harness=foundations`。
- 实际键盘操作：**PASS**。按用户授权，在 Codex In-app Browser 中通过原生键盘 press/click 操作，读取真实 activeElement 验证；不是在页面 dispatchEvent 合成 Tab。
- Button：Tab 到动作按钮，focus-visible outline 为 solid；Enter/Space 各触发一次，计数为 2。Tab 从打开按钮直接到刷新按钮，跳过 disabled/loading；实际点击 disabled/loading 后计数仍为 2。勾选动态禁用后，Tab 跳过动作按钮，native disabled 正确。
- 视觉/状态：disabled/loading opacity=0.52，disabled cursor=not-allowed；loading aria-busy=true，普通 disabled aria-busy=false。
- Dialog 初始焦点：通过 Tab 到 trigger、Enter 打开，焦点为关闭对话框按钮。
- 正向完整序列：关闭对话框 → 示例名称 Input → busy checkbox → 关闭 → 末尾焦点 Input → 关闭对话框。反向完整序列为上述逆序；两个边界均正常，未进入末尾 disabled tabindex=0 按钮。
- 普通 Esc：Dialog 消失，activeElement 恢复“打开基础对话框”。
- busy：用 Space 勾选后，两个关闭按钮 disabled；首个 focusable 为示例名称 Input，末尾为末尾焦点 Input，双向边界正常；Esc 后 Dialog 保持打开、aria-busy=true。取消 busy 后通过关闭按钮 Enter 关闭，焦点恢复 trigger。
- Console warning/error：空列表。
- 截图证据：`/private/tmp/maulink-phase2-browser-keyboard.jpg`（末尾 Input focus-visible）。
- 未把合成 KeyboardEvent 测试视为实际 native Tab 操作。

### Tauri Desktop

- 本轮 Browser 通过后已启动；Vite ready、Rust dev 编译成功、target/debug/maulink 已运行。用户已确认 Harness 显示及交付的键盘操作清单通过。
- 使用 frontend-v2 开发配置，并叠加 `/private/tmp/maulink-phase2-harness.conf.json`，仅将 devUrl 指向 `/?harness=foundations`；未修改正式 tauri.conf.json 或 frontend-v2 常规配置。
- 启动命令：`PATH=/private/tmp/maulink-tauri-tools/bin:$PATH cargo tauri dev --config src-tauri/tauri.frontend-v2.conf.json --config /private/tmp/maulink-phase2-harness.conf.json -- --locked`。
- WebView 实际键盘操作：**PASS（用户反馈）**。用户在收到完整清单后回复“通过”：Button focus/Enter/Space、disabled/loading 排除与动作阻止、Dialog 初始焦点、Tab/Shift+Tab loop、普通 Esc、busy Esc 规则、关闭后恢复 trigger 均登记通过。应用界面未由 AI 自动操作。
- 用户此前的 Phase 1 HMR/旧前端确认属于历史记录，不作为 Phase 2 键盘验收。

## 6. 视觉验收与剩余风险

- BaseButton 保留既有 `:disabled` 的 opacity/cursor 与 focus-visible 样式；loading 使用 native disabled 与 aria-busy=true。Browser 真实视觉/状态已确认；WebView 验收由用户确认通过。
- 当前自动测试可以验证 DOM 属性、action 和焦点边界逻辑，不能代替真实浏览器/WebView 的 native Tab 行为。
- Browser 与 Tauri WebView 键盘实际验收均 PASS，两个键盘阻塞已关闭。
- Light/Dark、中英文及多 viewport 全面视觉检查未在本轮执行，仅有本轮键盘焦点截图，不冒充全面视觉验收；本轮范围严格限定键盘问题。
- 本轮没有新增依赖；现有 Vitest 的未来配置导入提示仍记录为非阻塞风险。

## 7. 与旧前端差异

仅新增工程基础与开发 Harness，不迁移正式业务页面。没有改变 Rust DTO、IPC 字段、凭据存储、安全策略或 Terminal ACK/背压。新 stores 的领域边界在 README 中明确，不预建虚假业务状态或复制数据流。

## 8. 当前阻塞

无当前键盘阻塞。两个自动化失败已修正，17/17 测试、Browser 实际按键和 Tauri 用户验收全部通过。第 6 节的全面视觉与配置提示仍保留为未覆盖范围及非阻塞风险。

## 9. 是否达到退出条件

| 条件 | 结果 |
| --- | --- |
| type-check | PASS |
| test 100% | PASS（17/17） |
| build | PASS |
| Browser Harness 键盘实际操作 | PASS（Codex 内置浏览器实际按键） |
| Tauri Desktop 键盘实际操作 | PASS（用户明确反馈“通过”） |

**PASS**。五项退出条件全部通过。本轮按用户明确范围停在 Phase 2，不进入 Phase 3。

## 10. 提交内容与后续执行规则

用户在 Tauri 验收通过后明确授权：以后每个阶段 PASS 后，自动写好提交内容、提交并推送到主分支 `main`。应用界面继续由用户验收；Browser 可由 AI 在内置浏览器调试。阶段 BLOCKED 时停止，不提前进入下一阶段。

本次工作区包含此前已验收但尚未提交的 Phase 0 与 Phase 1，按范围分成两次提交后一起推送：

```text
fix(core): resolve monitor and sftp clippy warnings
refactor(frontend-v2): complete phase 1 and phase 2 foundations
```

第一笔包含 Phase 0 的等价 Core lint 修正与基线验收记录；第二笔包含精确 TypeScript 5.9.3 的 Vue/Vite/Tauri 开发工程、Typed/Mock IPC、Error Mapper、Base Components、键盘回归测试及 Phase 1/2 报告。提交说明记录 type-check、17/17 tests、build、Browser 与用户 Tauri 验收，并区分历史 Rust 检查与本轮检查。
