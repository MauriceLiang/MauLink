# MauLink Frontend v2

Phase 1 的 Vue 3 + TypeScript + Vite 空壳。旧 `frontend/` 和正式 Tauri 配置继续保留；本工程尚未接入 SSH、Terminal、SFTP、Monitor 或 Settings IPC。

## 安装与前端检查

Node.js 要求：`^20.19.0 || >=22.12.0`。首次在仓库根目录执行：

```bash
npm --prefix frontend-v2 ci
npm --prefix frontend-v2 run type-check
npm --prefix frontend-v2 run test
npm --prefix frontend-v2 run build
```

`package-lock.json` 固定依赖版本；后续通常使用 `npm ci`。TypeScript 精确固定为 `5.9.3`。Phase 2 已添加 Vitest、Vue Test Utils、jsdom 和 Mock IPC；当前阶段状态及失败项见 `docs/refactor/frontend-v2-phase-2.md`。

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

临时路径只适用于当前 macOS 开发机；其他环境使用 README 中已有的 Tauri CLI 安装方式。新配置使用 `io.maulink.frontend-v2.dev`，数据目录与正式 `io.maulink.desktop` 隔离。Core 初始化仍由已有 Tauri Adapter 完成，Vue 空壳没有调用业务 IPC。

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

开发模式的 `http://127.0.0.1:1420/?harness=foundations` 提供基础组件验收入口。该入口使用明确的 Mock IPC，不访问真实服务器；正式构建不启用此开发路由。当前还没有完成运行态验收，不能将入口存在视为测试通过。

组件应通过 `ipc/server.ts`、`connection.ts`、`terminal.ts`、`sftp.ts`、`monitor.ts`、`settings.ts` 的 typed facade 访问业务，不直接散落 `invoke`。facade 使用可注入的 `IpcClient`；生产默认使用模块化 Tauri API，测试注入 `createMockIpc`。只有 `app_get_info` 保留 Rust 当前不带 envelope 的签名。

DTO 直接 `import type` 复用仓库 `contracts/v1/`，不在新工程复制定义。`errors/mapper.ts` 处理 IPC 拒绝，`errors/presenter.ts` 根据模块 catalog 展示安全文案；未知异常和 Debug details 不直接展示。

后续状态模块必须按领域建立：Server 与 Terminal 状态分离；Terminal stdout 和文件字节不进入 reactive store；Transfer 只保存任务元数据/进度；Quick/Full Monitor 共用 snapshot 和控制器。Phase 2 尚未接入这些业务数据流，不预建虚假的 store 或业务页面。
