# MauLink V3.3 前端改造：P0 代码与原型审计

## 结论与范围

本轮只完成 P0。已核对更新工作树中的真实 Vue/Tauri 前端结构、关键 Store/IPC、设置字段、视觉测试入口，并把 30 项 V3.3 场景映射到现有代码。没有修改生产代码、Rust Core、生成契约、正式设置存储、旧视觉基线或原型文件。

**P0 尚未通过，不进入 P1–P8。** 两项前置条件未满足：

1. 原型 HTML 的 `file://` 页面被内嵌浏览器安全策略拒绝；本轮只能检查 HTML 源码和交付包中的 3 张静态 PNG，无法按 P0 要求逐页操作原型并核对交互状态。没有启动本地 HTTP 服务或改用未授权浏览器绕过限制。
2. 当前 `dev/app-updates` 与交付包指定的最新 `origin/main` 存在分叉，当前分支仍缺少 `origin/main` 的 1 个提交。P0 启动指令禁止自动合并主分支，因此没有擅自合并或变基。

交付包中的 `04_交给编码Agent的P0启动指令.md` 明确规定本轮仅做 P0、禁止生产代码改造并等待下一轮实施许可；主方案 §6 也将 P0 定义为“不修改生产代码”。因此本轮没有尝试先做部分 UI 改造。

## Git 与工作区基线

| 项目 | 结果 |
|---|---|
| 工作目录 | `/Users/mauriceliang/.codex/worktrees/0bd7/MauLink` |
| 当前分支 | `dev/app-updates`，与 `origin/dev/app-updates` 同步 |
| 当前 HEAD | `fdcb0a3a7cec481fb5b9768e2f2581a4b34db6d5` |
| 交付包指定基线 / 当前 `origin/main` | `20a0d897299319f8774ced5ae2526814f083f444` |
| merge-base | `1d8d8bd36d2b3d5fabb66633eaba2f8aad213aff` |
| 相对 `origin/main` | `git rev-list --left-right --count origin/main...HEAD` 输出 `1 2`：当前分支落后 1 个、领先 2 个提交 |
| 工作区状态 | 跟踪文件无修改；仅有既存未跟踪 `output/`，本轮保留不动 |
| Fetch | `git fetch origin main` 在共享 Git 元数据目录上首次遇到 `Operation not permitted`；经授权重试成功，`origin/main` 到达交付包要求的 SHA |

`origin/main` 上的新增提交为 Stable/Preview 应用内更新；当前 `dev/app-updates` 的两个独有提交分别涉及 GitHub 应用内更新发布流程及终端工作目录与远程文件视图。进入 P1 前须在既有 `dev/app-updates` 工作树内先整合最新 `origin/main`，复查 `TerminalWorkspace.vue`、`FilesPanel.vue`、更新安全门禁与相关测试。该整合不在本轮执行。

## 当前结构与数据流

```text
App.vue
└─ AppShell.vue
   ├─ Server Store ← typed server IPC → SQLite/Core 保存的服务器与分组
   ├─ ServerNavigation → 选择 / 右键动作
   ├─ Connection Store ← typed connection IPC → SSH 状态快照与挑战
   ├─ TerminalWorkspace（每个真实连接保留一个视图）
   │  ├─ Terminal Controller → xterm/PTTY 与 TerminalTabs 子标签
   │  ├─ FilesPanel → SFTP IPC；终端当前目录可同步到远程文件视图
   │  ├─ Transfer Store → SFTP transfer IPC/Channel
   │  └─ Monitor Store → 真实快照、历史与质量状态
   ├─ SettingsWorkspace → Settings API / revision / 自动保存
   └─ Updater Controller → 安装安全检查（连接、终端、传输、未保存设置）
```

- `App.vue` 只注入 IPC client；组合和导航在 `frontend/src/app/AppShell.vue`。
- `ServerNavigation.vue` 的普通行点击发出 `select`；`AppShell.selectServer()` 只更新 `selectedId`。真实连接由 `openServerView()` → `connections.start(server)` 发起，且当前命令面板选择服务器仍走 `selectServer()`。V3.3 的“点击树连接/聚焦”需要把两个入口收敛到同一真实动作。
- `frontend/src/stores/connections.ts` 的 snapshots 用 `server.id` 作 key；`start()` 对同服务器已有未结束快照直接返回。它提供 host key / 凭据挑战、取消、断开、错误及轮询；现状不支持同一服务器两个独立 SSH 工作区。
- `workspaces` 在 `AppShell.vue` 中由连接快照、终端 tabs、传输任务推导；现有渲染 key 为 `connectionId`，选中项却是 `server.id`。这不是独立的 UI 工作区标签状态。终端子标签由 `TerminalTabs.vue` / `terminal/controller.ts` 管理，不能与服务器工作区标签合并。
- `TerminalWorkspace.vue` 在 terminal/files/monitor 视图间切换，并保留终端 controller/xterm；关闭终端子标签与发起 SSH 断开是不同操作。断连根据设置或活动传输打开确认。
- `FilesPanel.vue` 调用真实 SFTP API，文件视图当前嵌在工作区布局中；`TransferPanel.vue` 按连接显示 Store 状态，现为文件区域内的面板。调整为全局底部抽屉时只能改变呈现与可见性，不能重建/取消任务。
- Monitor 使用共享 Store 的真实数据与 `stale`、`unsupported`、`error` 状态。不得为填充新布局而造数或将其压成单一在线状态。
- `AppSettings` 由 Core SQLite 设置记录与 revision 更新；TS 默认值与 Rust `AppSettings::default()` 当前均为 `theme=system`、`accentColor=blue`。`SettingsWorkspace` 使用 350ms 自动保存、revision 和离开/更新前提交保护。`TerminalSettingsSection` 与 server appearance 还承载终端背景及服务器级外观。
- 更新安装门禁汇总活动连接、终端、传输，并先提交设置；Shell 改造必须保留这些既有安全入口。

## V3.3 映射与迁移原则

| 原型区域 | 现有入口 | 计划中的最小迁移 | 必须保留的真实能力 |
|---|---|---|---|
| Shell / 顶栏 / 状态栏 | `app/AppShell.vue`, `TopBar.vue`, `Sidebar.vue`, `LocalBackendStatus.vue`, `styles/shell.css` | 重排现有 Shell；原生窗口拖动区、macOS 交通灯安全空间和状态栏继续使用真实平台信息 | Tauri 标题栏、命令面板、服务/连接/传输状态 |
| 服务器资源树 | `app/ServerNavigation.vue`, `Sidebar.vue`, `stores/servers.ts`, `stores/connections.ts` | 分组折叠、单一搜索、真实连接状态；点击与命令面板共用连接/聚焦动作 | CRUD、分组、过滤、Host Key/认证/失败/取消 |
| 极简空闲区 | `app/WelcomeView.vue`, `components/server/ServerList.vue`, `components/server-overview/ServerOverview.vue` | 拆分“零服务器引导”和“有服务器但零工作区”状态；新建 `IdleWorkspace.vue` 候选；常规主页移出重复卡片 | 加载/错误重试、ServerDialog CRUD、受控服务器诊断 |
| 工作区标签 | `AppShell.vue`, `TerminalWorkspace.vue`, `TerminalTabs.vue` | 新 UI 标签只保存工作区打开/选中状态；建议薄层 `DesktopWorkspaceTabs.vue` / UI Store，不复制 connection snapshots | 连接生命周期真实；xterm 子标签仍由 terminal controller 管理 |
| 终端 | `components/terminal/TerminalWorkspace.vue`, `TerminalTabs.vue`, `terminal/controller.ts`, `styles/terminal.css` | 适配工具栏和布局，不重写 controller | ANSI/UTF-8、IME、Ctrl+C、焦点、resize、ACK、终端会话释放 |
| SFTP | `components/files/FilesPanel.vue`, `FileMenu.vue`, `styles/files.css`, SFTP IPC | 文件行/路径栏密度和菜单视觉迁移 | POSIX 路径、分页、软链、权限、确认、终端目录同步 |
| 传输抽屉 | `components/files/TransferPanel.vue`, `stores/transfers.ts` | 提取为可折叠底部抽屉；只隐藏 UI，不卸载任务 Store | 多任务、连接来源、进度 Channel、取消竞态、清理错误 |
| 监控 / 检查器 | `components/monitor/MonitorView.vue`, `QuickMonitorPanel.vue`, `stores/monitor.ts`, `server-overview/*` | 紧凑呈现；可新增 Inspector 展示层，不复制采样逻辑 | 快照/历史、时间戳、stale/unsupported/error 与采集策略 |
| 设置 / 外观 | `app/settings/SettingsWorkspace.vue`, `TerminalSettingsSection.vue`, `terminal/preferences.ts`, `server-appearance` | 逐字段匹配 `contracts/v1/AppSettings.ts` 并迁移外观 | 自动保存、revision、更新通道、主题/强调色、密度、侧栏宽度、通知、终端外观及 per-server 覆盖 |
| 弹窗 / 图标 / 主题 | `dialogs/*`, `components/base/*`, `app/ShellIcon.vue`, `styles/tokens.css`, `themes.css`, `components.css` | 复用现有 Reka UI/Base 组件；统一图标审计与 Graphite 中性色 token | 可访问性、危险确认、用户 accent、浅/深/跟随系统 |

原型 HTML 是交互规格参考，不是产品数据源。其 `innerHTML`、全局状态、LocalStorage、假连接/延时/SSH/SFTP/监控数值不得迁入正式 Vue。组件名或 CSS 高度只作为映射线索，不能取代真业务状态。

## 三项待产品确认的行为建议

1. **关闭服务器工作区**：建议在无活动传输时明确关闭该工作区并断开 SSH；存在活动传输时先展示传输影响与确认，默认返回工作区，用户显式确认后才按现有断连流程停止任务/断开。关闭 xterm 子标签只关闭该终端，不应因此自动断开 SSH。
2. **同服务器多独立连接**：本轮不支持。服务器树单击发起唯一真实连接或聚焦已有工作区；连接中重复点击只聚焦等待状态，不重复启动。双击不新建 SSH，可复用同一“连接/聚焦”动作（或由产品明确只保留单击交互）。需要多个独立 SSH 时另立连接模型需求。
3. **Graphite 默认与用户数据**：建议 Graphite 作为新安装的中性视觉默认；持久化仍以 Rust Core SQLite 的 `AppSettings` 为单一事实来源，前端 `defaultSettings` 与 Core 默认保持一致。已有设置行和明确保存的 accent 值一律不重写。现有 Rust serde 对历史缺少 `accentColor` 的记录使用默认色；如改变新安装默认，需要为旧记录缺字段的兼容行为保留显式旧默认并加迁移测试。Graphite 的中性色 token 与 accent 自定义能力分开，不能把所有 accent 全局灰化。

这些为建议，需产品确认后才进入相应业务语义实现。P0 未对原型交互状态进行实测，不能把原型内某个双击或关闭动作描述为已确认行为。

## 最小端到端验收场景

1. **连接生命周期**：保存服务器树单击真实连接；首次/变化 Host Key、密码或密钥挑战、取消、错误、重试、成功、断连和重连；错误和 ready 状态准确。
2. **工作区与多服务器**：Web-01 / DB-01 交替切换仍保留连接；关闭一个工作区、关闭最后一个工作区、终端子标签全部关闭，以及设置页往返；无幽灵标签或假 ready。
3. **终端**：同一 SSH 下至少两个 xterm 子标签；ANSI/UTF-8 中文、连续输出、滚动、Ctrl+C、焦点模式、窗口 resize 与 controller detach/remount；关闭一个子标签不影响另一个。
4. **SFTP**：从终端当前目录进入文件区；路径进退、分页、长文件名、文件/目录/软链、双击和 Enter、右键动作与危险操作确认、失败恢复。
5. **传输**：上传/下载、多服务器来源、进度与状态；切页/折叠抽屉不中断；取消/失败/完成/清理；活动传输时关闭工作区与应用内更新安装安全门禁。
6. **监控**：真实 CPU/内存/磁盘/网络数据；正常、过期、unsupported、error 和无快照；文件/专注/设置切换时采集策略一致。
7. **外观与设置**：light/dark/system、Graphite 中性结构、预设与自定义 accent、标准/紧凑、侧栏宽度、终端字体/颜色/光标/图片/遮罩、server appearance 覆盖；修改后立即切页、重启后持久化且已有设置不变。
8. **更新门禁**：Stable/Preview 渠道保留；活动连接/终端提示与确认，活动传输阻止安装，未保存设置先提交，取消安装后可恢复正常工作。

## 现有验证入口与需维护的断言

- `frontend/package.json` 提供 `dev`、`type-check`、`test`、`build`。当前视觉 harness 的 Vite 地址为 `http://127.0.0.1:1420/`；`frontend/visual/cases.json` 有 35 个 Typed Mock IPC 场景，`frontend/visual/capture.mjs` 每页独立重载两次，候选不一致时不会覆盖基线。
- 现有视觉 manifest 与 `frontend/visual/verify.mjs` 校验 82 张旧截图；旧截图位于 `docs/refactor/screenshots/color-v1-review/`，是旧 MauLink Blue UI 的回归基线，不是 V3.3 当前实现截图。严禁将其当作 V3.3 对照或在未审阅时覆盖。
- 当前没有 V3.3 实现截图，也没有 V3.3 route/fixture。P1–P7 应新增独立候选与审核基线目录，给每个新状态绑定真实 UI 操作、视口、主题、语言和可复现 Mock 数据。
- `frontend/tests/shell.test.ts` 当前明确断言导航页不触发 SSH、Server Overview 与主页/添加入口行为；P2 需要定向更新这些行为断言，同时保留 IPC 安全、错误重试、settings 导航与 native drag 测试。
- `frontend/tests/terminal.test.ts` 已覆盖字节/ACK/UTF-8/ANSI、输入、controller detach/remount、关闭失败及设置应用；新 Shell 标签不能通过销毁重建 xterm 来通过视觉用例。
- `frontend/tests/files.test.ts` 已覆盖 POSIX 路径、分页、竞态、软链、操作确认、传输 Channel/轮询/取消/完成；底部抽屉迁移应新增“隐藏/切页不取消任务”与跨服务器来源场景。
- `frontend/tests/visual.test.ts` 维护 mock IPC 场景与安全边界；应增加连接/工作区/新主题视觉配置覆盖，不允许 mock 假 ready 绕过真实 store 状态流程。
- `SettingsWorkspace.vue` 与 Core 设置测试需覆盖新安装 Graphite 默认、legacy settings 缺字段、已有用户 red/custom accent、不修改已有记录、revision 冲突、延迟保存与更新安装提交。设置 contract 是生成文件，只能按 Core 型定义改动并重新生成。

**本轮未运行前端单测、类型检查、生产 build、真实 SSH/SFTP 或桌面构建。** 本轮没有生产代码改动；这些属于各阶段的实施门禁，不是本次 P0 源码审计的通过证据。

## P1–P8 最小文件范围（按交付包规划）

| 阶段 | 首要文件范围 | 边界 / 最小新增项 |
|---|---|---|
| P1 Token、图标、基础组件 | `styles/tokens.css`, `themes.css`, `components.css`, `components/base/BaseIcon.vue`, `app/ShellIcon.vue`, `theme/accent.ts` | 统一 token/icon；不改业务导航和设置持久化 |
| P2 Shell、服务器树、空态 | `app/AppShell.vue`, `TopBar.vue`, `Sidebar.vue`, `ServerNavigation.vue`, `styles/shell.css` | 可新增 `DesktopWorkspaceTabs.vue`, `IdleWorkspace.vue` 与薄 `stores/workspace-ui.ts`；不复制连接 Store |
| P3 终端工作区 | `components/terminal/TerminalWorkspace.vue`, `TerminalTabs.vue`, `styles/terminal.css` | 必要时薄 `SessionToolbar.vue`；不重写 `terminal/controller.ts` |
| P4 SFTP、传输抽屉 | `components/files/FilesPanel.vue`, `TransferPanel.vue`, `FileMenu.vue`, `styles/files.css` | `stores/transfers.ts` 仅必要只读接口，不复制任务状态 |
| P5 监控、检查器、状态栏 | `components/monitor/MonitorView.vue`, `QuickMonitorPanel.vue`, `styles/monitor.css`, `app/LocalBackendStatus.vue` | 可新增展示型 `Inspector.vue`；不复制采样逻辑 |
| P6 设置与个性化 | `app/settings/SettingsWorkspace.vue`, `TerminalSettingsSection.vue`, `styles/settings.css`, `styles/themes.css`, `terminal/preferences.ts` | 只有确认默认持久化策略后才改 Core 默认；保留现有字段与 revision |
| P7 视觉回归 | `frontend/visual/cases.json`, `capture.mjs`, `verify.mjs`, `frontend/tests/*`, `docs/redesign-v33/*` | 新建独立 candidate/baseline；旧 82 张基线只读 |
| P8 桌面发布验收 | `src-tauri/tauri.conf.json` 及各平台打包/验收配置（仅发现必要问题才改） | Mac/Windows 分别报告；不以浏览器测试替代桌面验证 |

该范围以交付包 §6 为准；每个阶段独立验收，不能预先一次性改完所有阶段文件。

## 风险、阻塞与未运行项

| 项目 | 级别 | 证据 / 当前状态 | 后续处理 |
|---|---|---|---|
| 无法交互核验原型 | P0 阻塞 | CUA 拒绝 `file://` URL；只审阅 3 张静态图与 HTML 源码 | 提供受支持的 `http(s)` 原型 URL，或明确接受静态参考审计后重新确认 P0 门禁 |
| dev 分支未包含最新 main | P0 阻塞生产阶段 | `origin/main...HEAD` 为 `1 2`，且 P0 禁止自动合并 | P1 前在既有工作树整合 `origin/main`，逐项复核冲突与测试；不新建分支 |
| Shell 同时承载多项业务 | 高 | `AppShell.vue` 同时组合连接、终端、传输、监控、设置、更新安全 | 渐进式拆分，不整体重写；每阶段独立通过业务和视觉门禁 |
| UI 工作区状态与真实连接耦合 | 高 | workspace 由 snapshot/terminal/transfer 推导；key 与 selected ID 不同 | UI tabs 独立存“打开/选中”，连接生命周期仍只由 connection Store 决定 |
| xterm 隐藏/卸载与 resize | 高 | xterm 有独立 controller、Channel 和 ACK 生命周期 | 用 `v-show`/稳定实例策略并验证 detach/remount 与 resize；不能以重建终端解决布局 |
| 传输抽屉化 | 高 | Transfer Store 管理异步 Channel、轮询、取消竞态 | 只改展示挂载与可见状态；切换页/收起不得取消或丢任务 |
| Graphite 与旧用户外观 | 高 | Core 和 TS 默认现为 system + blue；历史 JSON 缺字段由 serde 默认值兼容 | 先确定仅新安装的默认策略；增加兼容/迁移测试，保留现有用户的显式和 legacy 值 |
| 跨平台窗口外观 | 中 | 原型为浏览器模拟标题栏；桌面窗口是 Tauri 原生 | 真实 Mac/Windows 窗口分开验证，不照搬假交通灯 |
| 浏览器/Web、桌面与真实服务覆盖面不同 | 中 | 现有视觉 harness 使用 Typed Mock IPC | 分开报告 mock Web、Tauri container、真实 SSH/SFTP 与跨平台验证 |

### P0 门禁状态

- 代码结构、接口边界、差异和风险审计：**完成**。
- 验收矩阵更新：**完成**；逐项标注来源、代码证据、状态、负责人和缺口，见 [`01-parity-matrix.csv`](./01-parity-matrix.csv)。
- 旧视觉基线只读：**遵守**，未运行任何会写 baseline 的命令。
- 原型源码与静态参考审阅：**部分完成**；源码和三张 PNG 已核对。
- 逐页交互操作和 V3.3 原型/当前 Vue 同状态截图对照：**未验证**。
- P0 总状态：**未通过，等待阻塞解除与审核**。

进入 P1 的条件：能在支持的浏览器表面打开并操作原型（或产品明确将本轮 P0 验收降为静态审计）、审阅本报告及三项行为建议，并在既有 `dev/app-updates` 工作树中整合最新 `origin/main`。在这些条件满足前，不修改生产实现。
