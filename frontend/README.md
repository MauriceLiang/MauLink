# MauLink Frontend

Vue 3 + TypeScript + Vite 正式前端。Phase 12 将迁移工程从 frontend-v2 切换至 frontend；既有 Rust Core 与 contracts/v1 不变。TypeScript 精确固定 5.9.3。阶段报告及平台限制见仓库 docs/refactor/；用户采用 Browser 门禁继续，Phase 6 既有验收缺口保留。

## 安装与检查

Node.js 要求：`^20.19.0 || >=22.12.0`。仓库根目录执行：

```bash
npm --prefix frontend ci
npm --prefix frontend run type-check
npm --prefix frontend run test
npm --prefix frontend run build
```

## Browser 与 Tauri 开发

```bash
npm --prefix frontend run dev
# 或先停止独立 Vite，再运行桌面：
cargo tauri dev
```

Browser 地址 `http://127.0.0.1:1420`；普通入口需要 Native IPC，Browser 测试通过 DEV-only Harness，例如 `?harness=visual&page=servers&theme=light&locale=zh-CN`。Harness 不进入生产包。

Tauri CLI 自动发现 frontend/package.json 并在该目录执行 npm run dev / npm run build。模块化 @tauri-apps/api 不依赖全局 window.__TAURI__；正式 CSP 保持原策略，devCsp 仅额外允许本机 HMR WebSocket。系统窗口拖动权限仅授予 main。

```bash
cargo tauri build --bundles app -- --locked
# 可选隔离资料测试，仍使用当前 Vue 前端：
cargo tauri dev --config src-tauri/tauri.frontend-v2.conf.json
```

正式 identifier 为 io.maulink.desktop，继续使用原正式数据目录；隔离配置 io.maulink.frontend-v2.dev 继续可用。不会自动将隔离配置的资料复制到正式目录。旧前端可从切换前提交 f75d21a 的 frontend/ 恢复，详见迁移总结。

## RustRover Run Configuration

创建 Cargo Run Configuration，名称 MauLink Desktop，Working directory 为仓库根目录，Command 为 `tauri dev`。构建配置 Command 为 `tauri build --bundles app -- --locked`。隔离资料配置用 `tauri dev --config src-tauri/tauri.frontend-v2.conf.json`。先停止独立 Vite，避免固定 1420 端口冲突。

本机若 CLI 未加入 PATH，可在 Environment variables 中把 /private/tmp/maulink-tauri-tools/bin 加到原 PATH 前，保留已有内容；临时路径仅适用当前开发机。也可在 Terminal 使用 `PATH=/private/tmp/maulink-tauri-tools/bin:$PATH cargo tauri dev`。单独 cargo run 不会启动 Vite/HMR。

静态 IPC 调试页仍通过 `cargo tauri dev --config src-tauri/tauri.harness.conf.json` 启动；该配置关闭继承的 Vite hooks/devUrl，单独启用全局 Tauri API，产品正式窗口不启用。

> 下列 Phase 1–11 条目为迁移历史；早期空壳/旧入口验收步骤已由 Phase 12 正式切换取代。最新状态见 Phase 12 报告，开发命令使用当前 frontend 路径。

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

Phase 3 建立真实 Typed IPC 的只读 Shell，完整验收证据见阶段报告。浏览器没有 Tauri 时显示安全的本地服务不可用状态，不伪造后端就绪。当前 Phase 4 已启用服务器与分组管理，设置仍禁用；Phase 5 的选中服务器页新增连接状态与操作。

开发入口 `http://127.0.0.1:1420/?harness=shell&state=empty&theme=light` 使用 Mock IPC。右下角“Mock IPC”控件可切换空列表/有服务器、Light/Dark；仅限开发构建，不访问真实服务器或持久化资料。截图与检查结果见 `docs/refactor/frontend-v2-phase-3.md`。

Shell 沿用旧版 48px Topbar、30px Statusbar；Sidebar 在宽屏为 236px，1080px 及以下为 220px，900px 及以下为 190px。最小宽度仍为 860px，窄屏隐藏 Home 图标并保留品牌键盘入口，macOS 顶栏留出 78px/68px 原生交通灯区域。Cmd/Ctrl K 聚焦全局搜索，两处搜索同步；About 使用 Phase 2 Dialog 和焦点恢复。

Tauri 验收先运行默认新前端，核实真实本地服务和 app version；随后可通过临时 devUrl overlay 运行 Shell Mock Harness，检查两种数据状态及主题。frontend-v2 专用配置保留 main 业务 capability，仅补充 main 窗口的 core:window:allow-start-dragging，让 TopBar 空白区域支持原生拖动。正式配置、共享权限、窗口参数与默认旧入口均不改变。

每阶段完成全部验收后自动写提交说明、提交并推送到 `main`；阶段 BLOCKED 时停止；用户已取消后续 Tauri 手动验收门禁。

## Phase 4 Server 管理与验收

服务器主页提供卡片、资料查看、编辑和删除确认；两处搜索共享 Store query，使用本地已加载的分页资料过滤。侧栏“管理分组”提供新建、重命名和删除确认；删除分组会保留服务器。服务器所属分组、跳板机、代理、保活和超时在新增/编辑的“高级”区设置。

编辑先调用 server_get 获取当前 revision；更新/删除携带 expectedRevision。冲突时保留表单并提供明确的重载入口，不自动覆盖。已有凭据明确区分 keep/replace/clear；密码与私钥口令只保存在当前表单，保存或关闭后清空引用，不写入浏览器存储。私钥仅通过 local_file_select 返回的临时 token 引用。

Browser Mock 入口：`http://127.0.0.1:1420/?harness=servers&state=servers&theme=light`。右侧“Mock Server CRUD”可切换主题、模拟 ServerInUse/RevisionConflict/未知错误，以及延迟写入 10 秒验证 busy。资料仅保存在内存，重载会重置。

Desktop Native 验收入口：`?harness=servers&transport=native&theme=light`，使用真实 IPC、SQLite、系统凭据存储和文件选择器。通过临时 devUrl overlay 启动，不改正式配置。开发专用的回环占用探针仅接受唯一的 `Phase4-占用验收`、`127.0.0.1:42424`、用户名 `phase4`、password、无保存凭据/代理/跳板机、120000ms 超时的配置；只调用现有 test-mode connection_start/cancel，验证真实 ServerInUse。Mock、Native Harness 和探针均不进入正式 bundle。

当前详细检查与 Desktop 操作清单见 `docs/refactor/frontend-v2-phase-4.md`。本阶段已全部 PASS，可进入 Phase 5 的 SSH Connection 与安全交互迁移。

## Phase 5 SSH Connection 与验收

服务器“查看”进入连接页，连接/取消/断开通过既有 Core。首次 Host Key 明确核对，Esc 拒绝；Changed 默认拒绝，更新信任需展开并确认独立核实。认证挑战仅使用一次性输入，不保存凭据。错误诊断默认折叠，Retry 遵循 error.retryable，变化指纹不自动重试。已连接表示 SSH ready；Phase 6 工作区实现已接入，验收状态见下文。

Browser Mock：`http://127.0.0.1:1420/?harness=connections&theme=light`，可选择下一次连接场景及延迟回应。Native：`?harness=connections&transport=native&theme=light`，使用真实 IPC，通过临时 devUrl overlay 打开。Harness 不进入正式构建。

详见 `docs/refactor/frontend-v2-phase-5.md`。当前 Phase 5 按用户更新后的门禁 PASS，可继续 Phase 6。后续阶段通过自动化与 Browser 验收后自动提交推送 main；Tauri GUI 未实测时如实记录，不再请求手动检查。


## Phase 6 Terminal 工作区（BLOCKED）

已接入 xterm.js 5.5.0 / FitAddon 0.10.0、多终端、尺寸同步、专注模式与终端设置。正式数据流为 Tauri Channel → 非响应式 Terminal Controller → xterm.write → callback 后 ACK；Vue 只保存标签与状态元数据。默认关闭选中即复制；其偏好沿用旧浏览器存储 key。设置写入既有 Core SettingsService，保留非终端字段与 revision。

当前 type-check、66/66 tests、build、Rust workspace 与真实 OpenSSH terminal ignored test 通过。Browser 已验证真实 SSH、中文/ANSI、大量持续输出、隐藏终端继续消费、多终端、设置应用、专注模式与焦点恢复、top 显示/退出。Ctrl+C 的 Browser 按键操作尚未获得 Core 接受确认，vim、快速切换、关闭、断开和完整视觉检查尚未完成，用户于 2026-10-01 指示“直接推送进入”，允许保留上述验收缺口，先提交推送当前实现并进入 Phase 7；不将未验证项标为 PASS。

开发验收地址：`http://127.0.0.1:1420/?harness=terminal`。本机临时桥接工程为 `/private/tmp/maulink-phase6-bridge`，复用现有 Core 和 OpenSSH fixture，绑定 `127.0.0.1:1421`，仅接受 `http://127.0.0.1:1420` Origin。临时 SQLite / SSH key 不使用正式资料，Host Key 只信任与现场生成公钥指纹一致的 fixture。该桥接未纳入仓库，地址不是产品接口；Harness 与桥接地址不进入正式 bundle。

```bash
CARGO_TARGET_DIR="$PWD/target" cargo run --manifest-path /private/tmp/maulink-phase6-bridge/Cargo.toml --offline
# 另一终端
npm --prefix frontend run dev
```

Ctrl+C ACK 计数与 Ctrl+C keydown 字段仅用于开发诊断；不显示输入内容或 stdout。最新检查、限制和恢复点见 `docs/refactor/frontend-v2-phase-6.md`。用户已取消 Desktop 手动验收门禁；本阶段仍需 Browser 全部通过才能标记验收 PASS，但用户已明确授权先提交推送并进入 Phase 7。

## Phase 7 SFTP 与 Transfer

工作区“文件”视图使用 Core SFTP 分页，一次最多 200 行，支持目录导航、创建、重命名、删除确认和键盘菜单。远程路径保留 POSIX 语义；View/Edit 尚无后端接口，继续禁用。上传下载使用系统 picker token 与 Rust Transfer Manager，前端 Channel 仅消费进度元数据。取消须等待 Core 终态；断开活动传输需要显式勾选停止任务。

Browser 文件组件验收：`http://127.0.0.1:1420/?harness=files`；AppShell 组合验收：`?harness=files&workspace=1`，进入 Web-01 → 连接 → 文件。均为 DEV 内存 Mock，450 项目录、可重置恢复，不访问真实服务器或文件。场景控件支持完成、慢速取消、失败与取消文件选择。真实 SFTP 后端用仓库隔离 OpenSSH 测试验证。

最终 type-check、87/87 tests、build、Browser 和 4 项 OpenSSH SFTP 测试通过；按用户 Browser 门禁标记 PASS，Native GUI / 系统 picker 未实测。详细结果见 `docs/refactor/frontend-v2-phase-7.md`。Phase 6 的剩余验收问题继续保留。

## Phase 8 共享 Monitor

连接工作区的终端侧栏提供 Quick Monitor，“监控”入口打开 Full Monitor。两者使用 AppShell 的同一控制器与 Core Snapshot；前端统一读取快照及六类历史，不为卡片创建 timer。文件 / Home / 专注视图停止前端读取，Core 继续管理采集频率、固定脚本、backoff 和原生最小化生命周期。不可用与旧数据明确标识，手动刷新调用真实 Core 接口。

Browser 验收：`http://127.0.0.1:1420/?harness=monitor`，Web-01 → 连接。控件明确为 DEV Linux metrics fixture，提供五种质量、快照/历史 IPC 失败、计数与主题；不是实际 SSH/Linux 指标。最终 104/104 tests、type-check、build、11 项 Core Monitor 测试及 1 项真实 OpenSSH 监控/终端/传输并发测试通过。Native GUI、真实 Linux 成功指标和最小化/恢复未现场验证；按用户 Browser 门禁 PASS，详情见 `docs/refactor/frontend-v2-phase-8.md`。

## Phase 9：Settings / i18n / Command Palette

DEV 地址 `http://127.0.0.1:1420/?harness=settings`。真实设置通过 Rust SettingsService 更新；Harness 为显式内存 Settings/SSH fixture，不写正式资料。主题/语言/终端设置、模块 catalog、统一菜单和 Palette 已建立，115 项测试与 build 通过。Browser 工具无法完成终端焦点组合键，用户已在独立 Tauri release 应用完成 1–7 项实测，包含快捷键放行及重启持久化，并确认正常。本阶段 PASS，可进入 Phase 10；保留 Browser 工具限制记录。完整记录见 `docs/refactor/frontend-v2-phase-9.md`。

## Phase 10：固定 Browser 视觉回归

`?harness=visual&page=servers&theme=light&locale=zh-CN` 提供 DEV-only Typed Mock fixture。23 个 test route 与固定真实 UI 操作 recipe 见 `visual/cases.json`，截图自动化通过 CUA Browser 执行。Phase 10 当时记录了 184 张实现图和 160 张原型参考图；旧视觉归档已从当前仓库清理，现保留的颜色回归基线有 58 张。

默认回归不会覆盖旧图，缺失/变化会生成 candidate 并失败；审阅后才显式 update。`node frontend/visual/verify.mjs` 校验当前 58 张基线的 SHA256 与 JPEG 真实尺寸。详见 `visual/README.md`、`docs/refactor/frontend-v2-phase-10.md`、根目录 `design-qa.md`；Browser visual PASS 与完整 Native/platform QA 分开，既有能力缺口保留。

## Phase 11：响应式 Browser QA

四种视口及同一活动 Terminal 连续 resize / 专注模式焦点恢复通过；Phase 11 当时新增的 16 张中等/大视口截图均独立重复一致，相关归档现已清理。产品源码未改，沿用 Phase 10 的 125/125 tests、type-check 和 build。按用户 Browser 门禁 PASS，可进入 Phase 12；原生最小化/恢复与 Windows 真机仍未实测，完整平台发布验收未通过。见 `docs/refactor/frontend-v2-phase-11.md` 与 `design-qa.md` Page 18。

## Phase 12：正式切换完成

默认 Tauri 已使用本工程 dev/build output；旧入口可从 Git f75d21a 恢复。126/126 frontend tests、Rust workspace 检查与 68 tests、切换后 23 页 Browser 基线回归、macOS 正式启动及 app build/signature 通过。按用户 Browser 门禁 PASS，原 Phase 6 / Native / Windows 缺口继续保留，详见 `docs/refactor/frontend-v2-phase-12.md` 与迁移总结。
