# MauLink Frontend v2

Vue 3 + TypeScript + Vite 前端迁移工程。Phase 1/2/3/4 已通过；Phase 4 已接入 Server Home、Server/Group CRUD，自动化、Browser 与用户真实 Desktop 验收全部通过。旧 `frontend/` 和正式 Tauri 配置继续保留，SSH Connection、安全挑战、Terminal、SFTP、Monitor 与完整 Settings 尚未迁移。

## 安装与前端检查

Node.js 要求：`^20.19.0 || >=22.12.0`。首次在仓库根目录执行：

```bash
npm --prefix frontend-v2 ci
npm --prefix frontend-v2 run type-check
npm --prefix frontend-v2 run test
npm --prefix frontend-v2 run build
```

`package-lock.json` 固定依赖版本；后续通常使用 `npm ci`。TypeScript 精确固定为 `5.9.3`。Phase 2 已添加 Vitest、Vue Test Utils、jsdom 和 Mock IPC；阶段验收结果见 `docs/refactor/frontend-v2-phase-2.md`。

## Browser 开发

在仓库根目录执行：

```bash
npm --prefix frontend-v2 run dev
```

地址：`http://127.0.0.1:1420`。服务只监听本机回环地址，端口占用时直接报错，不会自动换端口。组件、CSS 的保存由 Vite HMR 更新。

## Tauri Desktop 开发

在仓库根目录执行：

```bash
cargo tauri dev --config src-tauri/tauri.frontend-v2.conf.json
```

此命令自动启动 Vite，不需要另开 `npm run dev`。若已运行 Browser 开发服务，请先用 Ctrl+C 停止，否则固定端口会冲突。

Tauri CLI 会自动发现 `frontend-v2/package.json`，开发和构建钩子分别在该目录执行 `npm run dev` 和 `npm run build`。

本机已有临时安装的 Tauri CLI，可使用：

```bash
PATH=/private/tmp/maulink-tauri-tools/bin:$PATH cargo tauri dev --config src-tauri/tauri.frontend-v2.conf.json
```

临时路径只适用于当前 macOS 开发机；其他环境使用 README 中已有的 Tauri CLI 安装方式。新配置使用 `io.maulink.frontend-v2.dev`，数据目录与正式 `io.maulink.desktop` 隔离。Core 初始化仍由已有 Tauri Adapter 完成。Phase 4 通过现有 serverApi 写入服务器与分组；产品入口不发起 SSH 连接。

打包新前端（不影响默认旧前端入口）：

```bash
cargo tauri build --config src-tauri/tauri.frontend-v2.conf.json
```

配置中的 `beforeBuildCommand` 会执行 Vite build，桌面资产来自 `frontend-v2/dist`。新前端不使用全局 `window.__TAURI__`，运行时所有前端依赖均从本地 bundle 加载；开发模式的 HTTP/WebSocket 仅用于本机 Vite 和 HMR。正式 CSP 继续继承原配置，开发 CSP 仅额外允许指定回环端口的 HMR WebSocket。

## RustRover Run Configuration

创建 Cargo Run Configuration：

- 名称：`MauLink Frontend v2`。
- Working directory：MauLink 仓库根目录。
- Command：`tauri dev --config src-tauri/tauri.frontend-v2.conf.json`。
- 本机若 `cargo tauri` 不在 PATH，在该配置的 Environment variables 中将 `/private/tmp/maulink-tauri-tools/bin` 加到原 PATH 前面，保留已有 PATH 内容。

也可直接在 RustRover Terminal 运行上述完整命令。不要运行单独的 `cargo run` 来验收 HMR，它不会自动执行 Vite 开发钩子。

## Phase 1 手动验收

1. 停止可能占用 1420 端口的开发服务，运行 Tauri Desktop 开发命令。
2. 确认窗口显示侧栏“服务器”和中央“前端工程已就绪”，不是旧版页面或空白窗口。
3. 在 `src/App.vue` 将中央标题改为“桌面 HMR 已通过”并保存。
4. 确认仍是同一个桌面窗口，标题自动变更，没有手动刷新、关闭重开或 Rust 重新编译。
5. 恢复原文“前端工程已就绪”并保存，确认窗口自动恢复。
6. 停止开发命令，在仓库根目录运行旧版 `cargo tauri dev`，确认仍可进入旧前端。

主题由 `index.html` 的 `data-theme="system"` 默认跟随系统；可临时改为 `light` / `dark` 保存检查基础 token，再恢复 `system`。这里只验收基础变量和空壳，完整主题/语言功能在后续阶段迁移。

反馈请包含：新桌面是否加载、保存后是否即时变化、Rust 是否重新编译、旧入口是否正常，以及失败时的完整终端错误。

参考：[Tauri Configuration](https://v2.tauri.app/reference/config/)、[Vite Server Options](https://vite.dev/config/server-options)。

## Phase 2 基础层与 Browser Harness

开发模式的 `http://127.0.0.1:1420/?harness=foundations` 提供基础组件验收入口。该入口使用明确的 Mock IPC，不访问真实服务器；正式构建不启用此开发路由。Phase 2 的 Browser 键盘与用户 Tauri 验收均通过，证据见阶段报告。

组件应通过 `ipc/server.ts`、`connection.ts`、`terminal.ts`、`sftp.ts`、`monitor.ts`、`settings.ts` 的 typed facade 访问业务，不直接散落 `invoke`。facade 使用可注入的 `IpcClient`；生产默认使用模块化 Tauri API，测试注入 `createMockIpc`。只有 `app_get_info` 保留 Rust 当前不带 envelope 的签名。

DTO 直接 `import type` 复用仓库 `contracts/v1/`，不在新工程复制定义。`errors/mapper.ts` 处理 IPC 拒绝，`errors/presenter.ts` 根据模块 catalog 展示安全文案；未知异常和 Debug details 不直接展示。

后续状态模块必须按领域建立：Server 与 Terminal 状态分离；Terminal stdout 和文件字节不进入 reactive store；Transfer 只保存任务元数据/进度；Quick/Full Monitor 共用 snapshot 和控制器。Phase 2 尚未接入这些业务数据流，不预建虚假的 store 或业务页面。

## Phase 3 Shell 与验收

Phase 3 建立真实 Typed IPC 的只读 Shell，完整验收证据见阶段报告。浏览器没有 Tauri 时显示安全的本地服务不可用状态，不伪造后端就绪。当前 Phase 4 已启用服务器与分组管理，设置仍禁用，选中服务器只查看资料。

开发入口 `http://127.0.0.1:1420/?harness=shell&state=empty&theme=light` 使用 Mock IPC。右下角“Mock IPC”控件可切换空列表/有服务器、Light/Dark；仅限开发构建，不访问真实服务器或持久化资料。截图与检查结果见 `docs/refactor/frontend-v2-phase-3.md`。

Shell 沿用旧版 48px Topbar、30px Statusbar；Sidebar 在宽屏为 236px，1080px 及以下为 220px，900px 及以下为 190px。最小宽度仍为 860px，窄屏隐藏 Home 图标并保留品牌键盘入口，macOS 顶栏留出 78px/68px 原生交通灯区域。Cmd/Ctrl K 聚焦全局搜索，两处搜索同步；About 使用 Phase 2 Dialog 和焦点恢复。

Tauri 验收先运行默认新前端，核实真实本地服务和 app version；随后可通过临时 devUrl overlay 运行 Shell Mock Harness，检查两种数据状态及主题。frontend-v2 专用配置保留 main 业务 capability，仅补充 main 窗口的 core:window:allow-start-dragging，让 TopBar 空白区域支持原生拖动。正式配置、共享权限、窗口参数与默认旧入口均不改变。

每阶段完成全部验收后自动写提交说明、提交并推送到 `main`；阶段 BLOCKED 时停止，应用界面由用户验收。

## Phase 4 Server 管理与验收

服务器主页提供卡片、资料查看、编辑和删除确认；两处搜索共享 Store query，使用本地已加载的分页资料过滤。侧栏“管理分组”提供新建、重命名和删除确认；删除分组会保留服务器。服务器所属分组、跳板机、代理、保活和超时在新增/编辑的“高级”区设置。

编辑先调用 server_get 获取当前 revision；更新/删除携带 expectedRevision。冲突时保留表单并提供明确的重载入口，不自动覆盖。已有凭据明确区分 keep/replace/clear；密码与私钥口令只保存在当前表单，保存或关闭后清空引用，不写入浏览器存储。私钥仅通过 local_file_select 返回的临时 token 引用。

Browser Mock 入口：`http://127.0.0.1:1420/?harness=servers&state=servers&theme=light`。右侧“Mock Server CRUD”可切换主题、模拟 ServerInUse/RevisionConflict/未知错误，以及延迟写入 10 秒验证 busy。资料仅保存在内存，重载会重置。

Desktop Native 验收入口：`?harness=servers&transport=native&theme=light`，使用真实 IPC、SQLite、系统凭据存储和文件选择器。通过临时 devUrl overlay 启动，不改正式配置。开发专用的回环占用探针仅接受唯一的 `Phase4-占用验收`、`127.0.0.1:42424`、用户名 `phase4`、password、无保存凭据/代理/跳板机、120000ms 超时的配置；只调用现有 test-mode connection_start/cancel，验证真实 ServerInUse。Mock、Native Harness 和探针均不进入正式 bundle。

当前详细检查与 Desktop 操作清单见 `docs/refactor/frontend-v2-phase-4.md`。本阶段已全部 PASS，可进入 Phase 5 的 SSH Connection 与安全交互迁移。
