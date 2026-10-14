# MauLink V3.3 前端改造：P0 代码与原型审计

## 结论与范围

P0 已完成。已核对最新 `origin/main`、当前 `dev/app-updates` 工作树中的 Vue/Tauri 前端结构、关键 Store/IPC、设置字段、视觉 Harness，并将 30 个改造区域映射到现有代码。完成 35 个固定 Vue 页面/交互场景的改造前截图，截图统一为 `988×756`、深色、简体中文、DPR 1；每个场景独立重载两次且图像哈希相同。原型的三张静态参照图已复制到本目录，原型主要交互也已实际操作核验。

P0 未修改产品运行代码、Rust Core、生成契约、正式设置存储或旧视觉基线。只修复了视觉采集脚本中已过期的设置保存步骤和背景图片预览选择器，确保其匹配当前的自动保存 UI 与实际预览元素。截图位于 `screenshots/p0-before/`，原型静态参考位于 `screenshots/prototype-reference/`。

用户已明确授权“按照文档逐个阶段自动执行”。因此，在每阶段验收通过后继续 P1–P8；继续使用现有 `dev/app-updates` 工作树和分支，不新建分支，逐阶段提交并推送，之后按用户既有流程重建应用；不创建 PR。

## Git 与工作区基线

| 项目 | 结果 |
|---|---|
| 工作目录 | `/Users/mauriceliang/.codex/worktrees/0bd7/MauLink` |
| 当前分支 | `dev/app-updates`；P0 完成后待提交/推送 |
| P0 审计起点 HEAD | `ee4a330ea18fd753395d42d282ca08b5faa56830` |
| P0 完成时 HEAD | `c86fb00`（整合 `origin/main` 后） |
| 交付包指定基线 / 当前 `origin/main` | `20a0d897299319f8774ced5ae2526814f083f444` |
| merge-base | 最新 `origin/main` 提交 `20a0d897299319f8774ced5ae2526814f083f444` |
| 相对 `origin/main` | `git rev-list --left-right --count origin/main...HEAD` 输出 `0 5`：当前分支包含 main 且领先 5 个提交 |
| 审计开始时工作区状态 | 仅有既存未跟踪 `output/`；已保留不动 |
| Fetch | `git fetch origin main` 在共享 Git 元数据目录上首次遇到 `Operation not permitted`；经授权重试成功，`origin/main` 到达交付包要求的 SHA |

`origin/main` 的 Stable/Preview 应用内更新已通过普通 merge 整合到现有 `dev/app-updates`；合并无冲突。合并后继续复查终端工作目录同步、远程文件视图、更新安全门禁和相关测试，不改写或丢弃这些已有能力。

## HTTP 原型交互核验

原型由用户通过 Python HTTP 服务提供，地址为 `http://127.0.0.1:8888/`，文件名为交付包中的 `index.html`。内置浏览器成功加载页面，标题为 `MauLink · Desktop V3.3 · Minimal Workspace`。本轮用可访问性树与可见页面核验了以下交互：

- 初始服务器树包含 Production 下的 Web-01、DB-01 和 Development 下的 Dev-01；搜索 `DB` 后仅显示 DB-01，Production 分组可折叠及展开。
- “新建服务器”打开名称、主机、用户名、端口表单；高级区提示正式版应保留已有代理、跳板、密钥和高级配置流程。未提交模拟表单。
- 命令面板列出连接服务器、打开终端/SFTP/监控、显示传输队列/检查器、切换资源树、添加服务器和打开设置等动作；Esc 可关闭。帮助说明确认连接、终端执行和文件操作都只是模拟。
- 选择 Web-01 后可切换终端、文件、监控；文件路径由 `/opt/app` 进入 `/opt/app/src`，点击 `package.json` 显示原型不打开真实远程文件。监控展示 CPU/内存/磁盘/网络示例值，明确标注模拟数据；检查器展示演示会话信息。
- 外观设置提供深/浅/跟随系统、六种低饱和配色、经典 accent、图标样式、密度、侧栏宽度、整体缩放、图标比例、圆角、通知位置及减少动效。终端设置含背景模式和图片、布局/遮罩预览等控件。
- 通用、安全与隐私、文件、网络、语言页显式声明部分功能只展示迁移边界，要求生产版保留现有业务设置；外观页也明确提示生产持久化映射到 Rust/Tauri，而不是用演示 LocalStorage 替代。

这些观察证明原型的主要路径可以交互，但不构成真实 SSH、命令执行、SFTP、文件读取、监控采样或设置持久化的验证。已在内置浏览器同一视口对照 Vue 当前实现与原型；Vue 仍显示左侧分组栏及主区服务器卡片，原型的服务器只在资源树中出现，空闲主区保持简洁。改造前 35 个固定 Vue 场景的截图、尺寸/主题/语言/DPR 元数据和独立重复哈希均归档在 `screenshots/p0-before/`；原型自带三张静态图归档在 `screenshots/prototype-reference/`。静态原型未提供 35 个生产功能等价状态，交互页面中的模拟 SSH/SFTP/监控数据只用于理解意图，不作为业务数据验收依据。

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

## 后续阶段采用的产品兼容边界

1. **关闭服务器工作区**：建议在无活动传输时明确关闭该工作区并断开 SSH；存在活动传输时先展示传输影响与确认，默认返回工作区，用户显式确认后才按现有断连流程停止任务/断开。关闭 xterm 子标签只关闭该终端，不应因此自动断开 SSH。
2. **同服务器多独立连接**：本轮不支持。服务器树单击发起唯一真实连接或聚焦已有工作区；连接中重复点击只聚焦等待状态，不重复启动。双击不新建 SSH，可复用同一“连接/聚焦”动作（或由产品明确只保留单击交互）。需要多个独立 SSH 时另立连接模型需求。
3. **Graphite 默认与用户数据**：建议 Graphite 作为新安装的中性视觉默认；持久化仍以 Rust Core SQLite 的 `AppSettings` 为单一事实来源，前端 `defaultSettings` 与 Core 默认保持一致。已有设置行和明确保存的 accent 值一律不重写。现有 Rust serde 对历史缺少 `accentColor` 的记录使用默认色；如改变新安装默认，需要为旧记录缺字段的兼容行为保留显式旧默认并加迁移测试。Graphite 的中性色 token 与 accent 自定义能力分开，不能把所有 accent 全局灰化。

依照用户对逐阶段自动执行的授权，实施采用上述安全边界；原型模拟的双击、关闭或数据值不会被解释为真实业务事实。若后续代码检查发现现有产品契约与此边界冲突，先保留既有契约并记录具体冲突，不做破坏性迁移。

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
| Vue 与原型逐页截图矩阵未归档 | P0 未完成 | 原型已在 `http://127.0.0.1:8888/` 交互；只做了 Vue 首页目视抽查，未逐项归档同状态截图 | 继续补齐矩阵要求的原型/Vue 同视口、主题、状态截图；不把旧 82 张基线当作 V3.3 证据 |
| dev 分支未包含最新 main | P0 阻塞生产阶段 | `origin/main...HEAD` 为 `1 3`，且 P0 禁止自动合并 | P1 前在既有工作树整合 `origin/main`，逐项复核冲突与测试；不新建分支 |
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
- 原型源码与静态参考审阅：**完成**；交付包源码、三张 PNG 和 HTTP 页面已核对。
- 主要原型交互：**已验证**；仅为浏览器模拟，未当作真实业务能力。
- Vue 改造前视觉基线：**完成**；35 个固定页面/交互场景，`988×756` 深色简体中文，DPR 1，两次独立重载截图哈希一致；见 `screenshots/p0-before/capture-dark-zh-CN-988x756.json`。
- 原型静态参考：**完成**；3 张交付参考图片见 `screenshots/prototype-reference/`；HTTP 原型主要可交互流程已实测，但假数据不作为产品行为证据。
- 视觉采集脚本对齐当前 UI：**完成**；设置控件按自动保存动作采集，背景图页定位到当前缩略图元素。
- 最新 `origin/main` 整合：**完成**；现有工作树无冲突，未新建分支。
- P0 总状态：**通过**。逐项原型细节和所有状态的精确像素差异仍将在 P1–P7 的视觉实现/回归中处理。

后续阶段按用户授权自动推进。涉及原型尚未明确的关闭工作区、连接状态或默认外观时，遵循主方案中安全的兼容路径：关闭前保留传输确认；同服务器维持唯一 SSH 连接并允许其下多个终端标签；默认中性色仅用于新安装，所有已保存的用户外观设置继续保留。

## P1｜视觉 Token、图标与基础控件

### 实施内容

- 浅色、深色和系统深色 token 已按 V3.3 Graphite 中性色更新；`--color-bg-active` 独立于 accent，服务器树选中态使用中性色。主题切换仍读取现有设置，蓝/红/自定义 accent 的 CTA、强调和焦点环仍使用用户颜色。
- `ShellIcon` 现在转发到 `BaseIcon` 的统一 24px SVG 几何；删除空闲页装饰性服务器图标和设置分类前置图标。按钮与输入控件统一 13px 文本、focus/hover/active 状态和过渡；服务器树 hover/active 不再位移或放大。
- 视觉 Harness 现在接受 `theme=system`，按实际系统偏好应用系统色值；旧 Harness 曾把该值降级为浅色，已补充解析与回归断言。
- 未改 `AppSettings`、Core 默认值、设置存储、连接流程或旧视觉基线。

### 验收结果

- `npm --prefix frontend run type-check`：通过。
- `npm --prefix frontend run test -- tests/components.test.ts tests/settings.test.ts tests/appearance-controls.test.ts tests/visual.test.ts`：4 个文件、47 项通过。Vitest 仍输出既有的 canvas mock 提示与 Vite `configLoader` 兼容性警告，不影响退出状态。
- Web Harness 以 `988×756`、简体中文，对系统、浅色、深色、红色 accent、`#FF2D8D` 自定义 accent 分别捕获终端工作区；每种条件独立重载两次且图像哈希一致。截图与 manifest 在 `screenshots/p1-candidate/{system,light,dark,red,custom}/`。
- 浏览器实测：浅色选中底色为 `#DFE4E6`，深色/当前系统深色为 `#30363B`；红色与自定义 accent 能改变 CTA/焦点环，选中底色保持中性。通过键盘 Tab 实际聚焦按钮，确认 `:focus-visible` 焦点环可见。
- P1 总状态：**通过**；在现有 `dev/app-updates` 工作树进入 P2。

## P2｜桌面 Shell 与唯一服务器资源树

### 实施内容

- 移除常规 ServerList 首页；有服务器时显示无图标的“暂无打开的会话”，无服务器时只显示一个添加入口；服务器诊断信息保留在右键菜单的显式详情路径中。
- 服务器树按分组显示，支持过滤、分组折叠和方向键移动焦点；树行展示来自 Connection Store 的连接状态。点击树节点和命令面板选中服务器统一走真实的 `connections.start()` / 已连接工作区聚焦路径。
- 新增轻量 `workspace-ui` 状态与服务器工作区标签。工作区 UI 状态以 `serverId` 唯一，连接生命周期仍由原 Connection Store 持有；终端子标签继续由既有 terminal controller 管理。
- 关闭工作区复用原断开确认、运行中传输保护和连接断开流程。最后一个工作区关闭后返回空闲态，失败/连接中工作区保留重试/取消入口；隐藏资源树不卸载工作区。
- 调整确定性 Web Harness 让不同服务器有独立的 connection ID，并更新受导航语义影响的 visual recipes、安全连接焦点测试和 Shell 测试。

### 验收结果

- Web Harness `?harness=visual&page=workspace-tabs`：树点击真实 mock 连接；Web-01/DB-01 形成两个独立 UI 工作区；命令面板搜索 DB-01 后激活对应工作区；生产分组可折叠/展开，方向键在树项间移动焦点；关闭最后工作区并确认断开后返回“暂无打开的会话”，无残留工作区。
- 无服务器 `?harness=visual&page=empty`：只出现一个可达的“添加服务器”主入口；服务器列表空态不再重复显示主页卡片。
- P2 补充 Web 复验：关闭 Web-01 后工作区归零；再次连接得到新的 connection ID，只出现新建的 `Terminal 3`，旧终端标签未被复用。Harness 现按服务器连接次数分配 ID，避免模拟出生产环境不会复用的连接标识。
- `npm run test -- tests/visual.test.ts tests/terminal-disconnect.test.ts tests/shell.test.ts tests/workspace-ui.test.ts`：4 个文件、28 项通过；覆盖重连 ID 唯一性与工作区关闭事件参数。
- `npm run test -- tests/shell.test.ts tests/workspace-ui.test.ts`：2 个文件、12 项通过。
- `npm run test -- tests/connections.test.ts`：1 个文件、17 项通过；连接安全弹窗测试通过显式详情入口进入。
- `npm run type-check`：通过。
- `npm run build`：通过。Vite 提示既有的大 JS chunk 超过 500 kB；未改动 bundle 分包策略。
- `git diff --check`：通过。
- 新版参考原型已在内置浏览器 `http://127.0.0.1:8888/` 打开并交互核对。当前验证是浏览器 mock，不代表真实 SSH/原生窗口验证。
- 原型 tabbar 的“新建连接”进入临时 SSH 工作区；现有 Core `ConnectionStartSource::Draft` 仅支持 `test` 模式，不能承载可持续工作区。本阶段不伪造该业务入口；如需该能力，需另增 Core connection lifecycle/API 设计，列为待决功能差异。
- macOS 交通灯原生安全区与窗口拖动不属于 Web Harness 可证明范围，留待 P8 桌面验收。
- P2 总状态：**Web 阶段通过**；原生窗口测试与“临时 SSH 工作区”能力不包含在本阶段通过范围。

## P3｜终端工作区与标签体系

### 实施内容

- 工作区标题栏增加终端/文件/监控分段切换与可折叠检查器，连接状态和断开操作留在同一工作区；Shell 工作区标签和 SSH 内终端标签仍分成两行、使用不同的状态与关闭动作。
- 专注模式隐藏服务器树、状态栏、工作区标题和快速检查器，保留终端工具栏与状态栏；工作区回到可见布局后重新 fit/focus，并显式刷新 xterm 已有行。
- 修复专注布局的高度链：专注时恢复 Shell 主区 flex 布局，让工作区内容区获得可计算高度；终端主区用单行 grid 填满可用高度。此前浏览器中 xterm 保留在无障碍树但视觉面板空白，复现后用工作树实时 Web 预览确认修复。
- 安全关闭已结束连接时释放该 connectionId 下所有本地终端 runtime 和标签；若 PTY open 在资源释放后才完成，则 best-effort 关闭新 PTY，避免遗留会话；断开请求失败时保留 ready 快照并展示错误。

### 验收结果

- 工作树 Vite Web Harness `http://127.0.0.1:1422/?harness=visual&page=workspace-tabs&theme=dark&locale=zh-CN`：专注模式截图可见原有中文 Shell 文本，终端从普通布局的 91×21 扩至 146×25；退出专注后原输出和快速检查器恢复。
- Web 复验显示工作区标题栏与终端子标签分行；终端/文件/监控切换正确，检查器可隐藏/恢复；设置页往返保留 `Terminal 1`、`Terminal 2` 及输出。
- 新建第二个终端后切回 `Terminal 1`，已有输出仍在；打开 DB-01 后出现独立工作区 `Terminal 3`，切回 Web-01 时两个原子标签和输出保留。隐藏资源树将终端从 91 列扩为 119 列，再显示恢复 91 列。
- 关闭 Web-01 时先出现断开确认；确认后 Web-01 从树移除已连接状态且其终端标签不再显示，DB-01 工作区和 PTY 标签仍保留。再关闭 DB-01 并确认后回到“暂无打开的会话”，工作区计数为 0。
- `npm --prefix frontend run test -- tests/connections.test.ts tests/terminal.test.ts tests/terminal-disconnect.test.ts tests/shell.test.ts tests/workspace-ui.test.ts`：5 个文件、58 项通过；覆盖断开失败保持 ready、终端释放、过晚完成的 open 清理、焦点刷新和检查器切换。
- `npm --prefix frontend run type-check`、`npm --prefix frontend run build` 与 `git diff --check`：通过。build 有现存大 chunk 提示；测试有既有 canvas mock / localStorage / Vite configLoader 提示。
- Web 结果来自 Typed Mock IPC，不代替真实 SSH/SFTP、输入法/OS Ctrl+C、macOS 原生窗口与其他平台测试；视觉截图矩阵仍按 P7 另行归档。P3 总状态：**Web 阶段通过**。

## P4｜SFTP 文件区与全局传输抽屉

### 实施内容

- 文件列表使用固定列布局，名称、大小、修改时间与原型保持左侧对齐；列表不再为每个工作区预留常驻传输栏。
- 将 `TransferPanel` 移到 `AppShell`，由状态栏传输计数按钮打开/收起。抽屉保持挂载并用 `v-show` 隐藏，关闭抽屉不会取消任务；打开时从所有 ready 连接加载传输历史。
- 任务按服务器名、方向、进度、状态与错误聚合；未知服务器以连接 ID 短码标识。失败任务继续呈现 Core 的可读错误和临时文件清理提示，活动任务保留取消操作，已完成任务可统一清理。
- 抽屉改为原型中的固定 180px 底部列表布局；目录打开键盘 Enter 可用，并修复“打开”按钮双击冒泡导致同一目录重复打开的问题。

### 验收结果

- 通过内置浏览器加载本地 Web Harness：`http://127.0.0.1:1422/?harness=visual&page=transfer&theme=dark&locale=zh-CN`。打开 Web-01 的文件视图后，可从状态栏展开全局抽屉；抽屉显示活动、失败、已完成三项任务，来源为 Web-01，进度、磁盘错误和取消按钮均可见。关闭/重开和文件区并列布局已在浏览器中核对。
- 与原型 `http://127.0.0.1:8888/` 对照后，将先前过高的卡片抽屉调整为原型 180px 高度的紧凑行列表；文件表格的大小、修改时间列改为原型左对齐。
- `npm --prefix frontend run test -- tests/files.test.ts`：1 个文件、25 项通过。覆盖分页游标、目录/软链接导航、上传下载文件选择、取消与终态竞争、失败清理提示、跨服务器任务聚合、抽屉隐藏保留活动任务、右键菜单和 Enter/双击行为。
- `npm --prefix frontend run type-check` 与 `git diff --check`：通过。
- Web 使用 Typed Mock IPC；没有真实 SSH/SFTP 主机。本阶段未用浏览器模拟真实文件读写；真实 SSH/SFTP 与原生桌面复测留待 P8。P4 总状态：**Web 阶段通过**。
