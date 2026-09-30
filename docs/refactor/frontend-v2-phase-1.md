# Phase 1 验收报告

检查日期：2026-09-30（Asia/Shanghai）

状态：**PASS：自动检查、Tauri Desktop 加载、可见 HMR、原标题恢复和旧入口运行复核均通过。允许进入 Phase 2。**

## 1. 完成项

- Phase 0 的 Desktop 启动及目标页面验收按用户反馈记录为通过。
- 建立独立 `frontend-v2/`：Vue 3、strict TypeScript、Vite、静态空壳、Design Token、system/light/dark 基础变量。
- 根据用户明确授权，将 TypeScript 精确固定为 `"typescript": "5.9.3"`，没有使用 `^` 或 `~`。
- 删除整个 `frontend-v2/node_modules`，更新 lockfile，再执行 `npm ci` 干净安装。
- 没有升级 Vue、vue-tsc、Vite 或其他依赖。对比前后 lockfile，仅 TypeScript 从 7.0.2 降至 5.9.3，并移除其 20 个不再需要的 TypeScript 7 平台包。
- 添加独立 Tauri 开发配置，使用 `io.maulink.frontend-v2.dev` 隔离正式数据目录；保留默认旧入口和正式配置。
- 修正新配置的 `beforeDevCommand` / `beforeBuildCommand` 为 `npm run dev` / `npm run build`。Tauri CLI 自动发现 `frontend-v2/package.json` 后在该目录执行 hook；原 `npm --prefix frontend-v2 ...` 会重复拼接路径。
- 已实际启动独立 Vite 并验证 HTTP 服务；随后停止该服务，让 Tauri 开发命令自动启动 Vite，避免固定端口冲突。
- 已实际执行新配置的 Tauri dev，完成编译并运行 `target/debug/maulink`；用户确认显示侧栏“服务器”和中央“前端工程已就绪”。随后修改 App.vue 可见标题，实际收到 HMR 更新消息；用户确认同一窗口自动变化，没有手动刷新或重开。原标题已恢复并再次产生 HMR 日志；用户确认标题自动恢复，以及旧前端首页、Add Server 和 Settings 正常。

## 2. 修改文件

```text
frontend-v2/package.json
frontend-v2/package-lock.json
frontend-v2/tsconfig.json
frontend-v2/vite.config.ts
frontend-v2/index.html
frontend-v2/src/main.ts
frontend-v2/src/App.vue
frontend-v2/src/styles/tokens.css
frontend-v2/src/styles/themes.css
frontend-v2/src/styles/base.css
frontend-v2/README.md
src-tauri/tauri.frontend-v2.conf.json
.gitignore
docs/refactor/frontend-v2-baseline.md
docs/refactor/frontend-v2-phase-1.md
```

`.gitignore` 仅为新工程增加 dist 忽略。前轮获授权的 2 个 Core Clippy 修正仍在工作区；本阶段没有追加 Rust 修改。`frontend/` 和正式 `src-tauri/tauri.conf.json` 无差异。

## 3. 新增依赖

| 名称 | 精确版本 | 用途和理由 |
| --- | --- | --- |
| `vue` | `3.5.43` | 手册指定的组件化 runtime |
| `vite` | `8.3.1` | 手册指定的 dev server、HMR 和本地 bundle 构建 |
| `@vitejs/plugin-vue` | `6.0.9` | Vue SFC 编译及 HMR，Vite 本身不处理 SFC |
| `typescript` | `5.9.3` | strict TypeScript；用户指定兼容版本 |
| `vue-tsc` | `3.3.11` | Vue 模板及组件类型检查；没有升级或替换 |
| `@types/node` | `26.6.3` | Vite 配置的开发类型 |

只有 Vue 进入应用 runtime，其余是开发工具；没有 UI 框架、Pinia、CDN 或新增 Rust runtime。旧 ES Modules 工程没有提供上述 Vue/SFC/构建能力。npm registry / peer 信息用于选型，最终兼容性以实际检查为准。

Node 要求 `^20.19.0 || >=22.12.0`，本机为 `v26.10.0`。Windows 平台尚未验证。`npm ci` 输出 48 packages added、49 packages audited、0 vulnerabilities。npm 还提示可选 `fsevents@2.3.3` 的 install script 未获 allowScripts 覆盖；没有自行批准额外脚本，开发服务及文件变化的实际 HMR 已通过本阶段验收。

## 4. 自动化检查

### Frontend

| 检查 | 结果 |
| --- | --- |
| 删除依赖并干净安装 | PASS：删除 node_modules，`npm install --package-lock-only --ignore-scripts` 后 `npm ci` 退出码 0 |
| manifest / lockfile / installed TypeScript | PASS：三处均为精确 `5.9.3` |
| 其他依赖版本比较 | PASS：除 TypeScript 及其移除的平台包外，所有 locked package 版本保持原值 |
| `npm ls --depth=0` | PASS：6 个直接依赖版本与清单一致 |
| `npm run type-check` | PASS，退出码 0，真实执行 vue-tsc |
| `npm run build` | PASS，退出码 0，15 modules transformed |
| 构建产物 | HTML 0.42 kB、CSS 2.61 kB、JS 61.52 kB；均本地资产 |
| `npm run dev` | 首次沙箱内 FAIL：`listen EPERM 127.0.0.1:1420`；自动审批允许绑定回环端口后重试 PASS |
| Vite HTTP | PASS：`/`、`/src/App.vue`、`/@vite/client` 均 HTTP 200，入口和标题内容正确 |
| HMR transport | PASS：订阅真实 Vite WebSocket 后修改标题，收到 `/src/App.vue` 的 `js-update`，重新读取模块包含新标题 |
| `git diff --check` | PASS |
| `git diff --exit-code -- frontend src-tauri/tauri.conf.json` | PASS：旧入口及正式配置未修改 |
| `npm run test` | 未配置；Phase 1 空壳暂无业务逻辑，框架及 Mock IPC 按手册在 Phase 2 建立 |

### Rust

- `cargo check --workspace --all-targets --locked`：本轮 PASS，退出码 0。
- fmt / clippy / test：本阶段没有追加 Rust 改动；Phase 0 修正后均通过，68 passed / 11 ignored。本阶段未重复执行这三项。
- `cargo tauri dev --config src-tauri/tauri.frontend-v2.conf.json -- --locked`：新配置的真实 Rust dev 编译成功（日志 14.26 s），已执行 desktop 二进制；不能把进程启动等同于窗口和 HMR 验收通过。

## 5. Desktop 实际验证

启动命令（本机临时 CLI 已存在）：

```bash
PATH=/private/tmp/maulink-tauri-tools/bin:$PATH cargo tauri dev --config src-tauri/tauri.frontend-v2.conf.json -- --locked
```

日志确认：

```text
Running BeforeDevCommand (`npm run dev`)
VITE v8.3.1 ready
Local: http://127.0.0.1:1420/
Finished `dev` profile
Running `.../target/debug/maulink`
```

Tauri 监听的 Rust 目录为 `src-tauri` 和 `crates/maulink-core`。没有通过 CUA 或其他 GUI 自动化操作应用。用户已确认窗口显示预期内容，Desktop 加载登记 PASS（用户反馈）。将 App.vue 可见标题临时改为“桌面 HMR 已通过”后，真实 Vite WebSocket 返回 `js-update`，HTTP 模块内容已更新，Tauri 日志只出现 `[vite] (client) hmr update /src/App.vue`，没有新的 Rust 编译输出。用户随后确认同一个窗口自动变为新标题，没有手动刷新或重开窗口。可见 HMR 登记 PASS（用户反馈 + 服务日志），原有 TypeScript 7 兼容错误不再出现。

## 6. 视觉验收

- Light / Dark / system：变量和 CSS 已建立，运行态观察待用户反馈。
- 中文：空壳固定中文标题；完整 English catalog 不在本阶段范围。
- Viewport：继承正式默认窗口及最小尺寸；没有进行多尺寸或像素级回归。
- 没有生成可见截图证据或宣称完整视觉 QA 通过。

## 7. 与旧前端差异

新工程仅有空壳，没有业务 IPC。旧前端继续提供 Server / SSH / Terminal / SFTP / Monitor / Settings。新配置不使用全局 Tauri 注入，开发 CSP 只额外允许指定本机端口的 HMR WebSocket；生产 CSP 和现有权限继承正式配置。

## 8. 当前阻塞与用户反馈

TypeScript 7 / vue-tsc 兼容问题已通过用户指定的 5.9.3 修正并验证。本阶段阻塞已解决，所有目标检查通过。执行记录：

1. 用户已确认新窗口显示侧栏“服务器”和中央“前端工程已就绪”。
2. AI 已将 App.vue 可见标题改为“桌面 HMR 已通过”并保存，收到真实 HMR 更新消息。
3. 用户已确认同一个窗口自动变化，未刷新/重启；日志未出现 Rust 重新编译。
4. AI 已恢复原标题，Vite 再次记录 App.vue HMR，用户确认窗口自动恢复。
5. 默认旧配置仅叠加临时 identifier/productName 隔离数据，编译及进程启动通过；用户确认旧窗口首页、Add Server 和 Settings 正常。

旧前端启动命令：

```bash
PATH=/private/tmp/maulink-tauri-tools/bin:$PATH cargo tauri dev --config /private/tmp/maulink-migration-baseline.conf.json -- --locked
```

临时 overlay 只含 `identifier=io.maulink.migration.baseline.20260930` 与 `productName=MauLink Migration Baseline`，不覆盖 frontendDist、窗口、权限或正式 CSP。默认旧前端 dev 编译通过（11.11 s），已运行 desktop 进程。

用户明确反馈“标题已恢复，旧版首页及两处入口正常”。旧文件完整性检查及运行复核均通过；应用操作证据标明来自用户反馈。

## 9. 是否达到退出条件

**PASS**：8 项目标已全部完成。依赖精确版本与 lockfile 一致；type-check、build、Vite HTTP、Tauri dev 启动通过；用户确认可见 HMR、原标题恢复及旧前端入口正常。阶段门槛满足，允许进入 Phase 2。

## 10. 推荐 Commit

```text
feat(frontend-v2): bootstrap vue typescript vite desktop shell
```

该阶段工程与报告按用户后续授权，随 Phase 2 验收通过后一起提交到主分支；提交范围见 Phase 2 报告。

参考：[Tauri Configuration](https://v2.tauri.app/reference/config/)、[Vite Server Options](https://vite.dev/config/server-options)。
