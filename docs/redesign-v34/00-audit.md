# MauLink V3.4.1 前端改造审计

## P0 基线

- 工作树：`/Users/mauriceliang/.codex/worktrees/0bd7/MauLink`
- 分支：`dev/app-updates`
- 基线：`a28f64b7e2d150d62e09734237d064b52e389167`（`origin/main`；与交付包标注的源码基线一致）
- 开始时工作区：只有已存在的未跟踪 `output/`，不属于本次改造。
- 交付包称的改造范围：Vue/Tauri 前端表现层。此次不改 Rust Core、typed IPC、SQLite 契约、SSH/SFTP/Monitor Store。
- 现有 V3.3 工作区、菜单、Terminal/SFTP/Monitor、Settings 自动保存和安全流程均已落地；本次只做视觉与组件密度精修。

在同步 `origin/main` 后，`dev/app-updates` 快进到 `a28f64b`，没有重置或丢弃工作区文件。旧 `docs/redesign-v33/` 及截图均保留为只读参考。

## 改造前 Web 复现

使用工作树 Vite Mock Harness 和 Codex 内置浏览器：`http://127.0.0.1:1422/`，CSS 视口约 `988×756`，DPR 1，Dark、简体中文。复现添加服务器后记录：

| 场景 | 改造前观察 | 计划处理 |
|---|---|---|
| 添加服务器 Basic | 框宽约 640px；控件、边角和主按钮仍偏大；基础与高级 Tab 正常；测试/取消/保存已有正确分区 | P1 统一密度、半径和默认操作层级；P2 固定标题/Tab、滚动内容及 Footer |
| 添加服务器 Advanced | 连接参数底部字段被 Footer 遮挡；两个配置 Tab 当前高度内容不同 | P2 中间独立滚动并固定操作区；保持 Advanced 为独立 Tab |
| 编辑服务器 Appearance | 通过现有 `ServerAppearancePanel.vue` 提供标签颜色和终端覆盖；不重写图片存取 | P2 仅收敛布局，保留全局继承、预览、图片 ID 和服务器级覆盖 |
| 普通断连/删除/Host Key | 现有专用流程分别承载确认、危险操作和身份核验 | P3 分别调整视觉，不合并安全语义 |
| 无会话/零服务器/工作区 | V3.3 资源树及极简空闲工作区已存在 | P4 仅检查细节，避免重复建设 |
| Files/Transfer/Monitor/Settings | SFTP、全局任务抽屉、监控质量状态与设置自动保存已存在 | P5 只校准控件和密度 |

改造前全量视觉参考、状态和采集元数据见 `docs/redesign-v33/screenshots/` 及 `frontend/visual/matrix-v33.json`。这些是 V3.3 证据，本轮会在 `docs/redesign-v34/` 建立独立候选，不覆盖旧图。

## 高度自定义能力映射（P0 发布阻塞项）

| 字段/能力 | 当前控件与生效路径 | 持久化来源 | 作用域/视觉验收 | 本次处理 |
|---|---|---|---|---|
| `theme`、`accentColor`、`customAccentColor` | `SettingsWorkspace.vue` 外观分类；`theme/accent.ts` 将 accent 派生为 CSS 变量 | `settings_get` / `settings_update`，Core 设置记录与 revision | Light/Dark/System 与 blue/red/custom；结构色保持中性 | 保留字段和用户颜色变量，只增加中性动作 Token |
| `uiDensity`、`sidebarWidth`、`toastPosition` | `SettingsWorkspace.vue`；根 CSS 密度变量、Shell 宽度变量、Toast viewport 定位 | Core 设置记录与 revision | Standard/Compact、三档侧栏、六个通知位置，需即时生效 | 保留选项和应用路径，校准标准/紧凑控件尺寸 |
| `appIconStyle`、`language` | 设置页图标预览与语言选择；i18n locale 切换 | Core 设置记录与 revision | Light/Dark 图标、简中/英文 | 不删选项，不改值 |
| `terminalFontFamily`、`terminalFontSize`、`terminalLineHeight`、`terminalCursorStyle`、`terminalCursorBlink`、`terminalScrollbackLines` | `TerminalSettingsSection.vue`、`TerminalAppearancePreview.vue`；Terminal Controller/xterm | Core 设置记录与 revision | 预览与真实终端，resize/输入焦点保持 | 仅统一视觉；不改终端控制器 |
| `terminalThemeMode`、`terminalCustomColors` | `TerminalThemeModeSelector.vue`、`TerminalAppearanceEditor.vue`、`appearance.ts` / `theme.ts` | Core 设置记录与 revision | `followApp`/light/dark/customColor/image；背景、前景、光标、选区独立 | 保留全部模式、颜色和预览 |
| `terminalBackgroundImage` | `TerminalAppearanceEditor.vue`、`TerminalBackgroundPositionPicker.vue`、预览；背景图 API 以 `imageId` 读取 | Core 图像资源 API + Core 设置记录/revision | fit、position、imageOpacity、overlayKind、overlayOpacity、blurPx；切页/重启恢复 | 不改资源 ID、导入/清理或本地路径策略 |
| `copyOnSelect` | `TerminalSettingsSection.vue` 复选框，Terminal Preferences | `maulink.terminal.copyOnSelect` 本地偏好键 | 即时复制偏好，禁用浏览器存储时保持安全默认 | 单独保留，不并入 AppSettings |
| `ServerAppearance.labelColor`、`environment` | `ServerAppearancePanel.vue` | `server_appearance_*` IPC、按 `serverId` 的 Core 记录与 revision | 两台服务器值隔离并回读 | 保留字段、颜色选择器与环境选择 |
| `terminalOverrideEnabled`、`terminalAppearance` | `ServerAppearancePanel.vue` + `TerminalAppearanceEditor.vue`；`resolveTerminalAppearance()` 按 serverId 解析 | Server Appearance IPC/Core | 继承全局与单服务器覆盖隔离；imageId、模式、颜色及图片参数 | 不合并全局和服务器级对象 |
| 其他 `AppSettings` | Settings 的 General/Security/Files/Network/Update 等分类 | Core 设置记录与 revision | 连接确认、安全策略、文件显示、下载目录、GeoIP、更新通道等不受 CSS 改造影响 | 不改设置契约或自动保存逻辑 |

本次不添加 V3.3 HTML 演示中的 UI 整体缩放、图标倍率或任意圆角值：当前生成契约 `contracts/v1/AppSettings.ts` 没有对应持久字段。它们不属于此次既有能力保全范围。

## P1 基础组件与 Design Tokens

- 新增按用途命名的控件/弹窗半径与标准/紧凑高度 Token；按钮、输入框、下拉选择和颜色控件共用基础尺寸。
- 新增 `neutralPrimary`，与原 `primary` 的用户 accent 路径并存；不改 `--color-primary`、焦点环或终端颜色解析。
- `BaseDialog` 增加可组合的 `confirmation` / `form` / `security` 外观语义；仍由 Reka Dialog/AlertDialog 提供 modal、焦点、Esc 与关闭行为。
- 确认弹窗与表单外框尺寸收敛到建议范围；确认/表单语义变体可单独控制留白和分割线，业务模板迁移留在 P2/P3；Reka Select/Menu/Popover 继续使用 Portal 与定位逻辑。
- 更新交互 Harness 展示中性主动作和确认弹窗变体，便于浏览器复核。

### P1 验收

- Web `?harness=interaction&theme=dark`：已观察中性按钮与 accent 主题按钮同时呈现；输入、选择、disabled、loading 控件可见；下拉列表打开后焦点在首个可选项。
- Web 确认弹窗：标题/内容/操作区紧凑，无贯穿横线；Esc 可关闭，焦点返回“打开 Dialog”触发按钮。
- Web `?harness=visual&page=add-server&theme=dark&locale=zh-CN&accentColor=red`：红色仍用于焦点、Tab 和保存动作，结构背景保持深色中性。
- `npm --prefix frontend run type-check`：通过；完整类型检查和测试结果汇总在 P7。
- P1 总状态：**Web 阶段通过**；弹窗业务模板迁移与 860×640 窄窗口核验进入 P2。

## P2 服务器表单

- `ServerDialog.vue` 保留 Basic / Advanced / 编辑态 Appearance 三个既有 Tab、表单提交、Test Connection、凭据 keep/replace/clear 和图片/外观组件；只将表单语义传给 `BaseDialog`，保存按钮切换到独立 `neutralPrimary` 变体。
- `.server-dialog` 使用 580px 最大宽、620px 目标高并受视口高度限制；标题/Tab 和 Footer 固定，中部面板独立滚动。标签列调整为 108px，主机/端口与跳板机/端口继续同行，端口列 104px 且无独立标签。连接参数两个字段并排显示，标签位于输入框上方。
- Web Mock `?harness=visual&page=add-server&theme=dark&locale=zh-CN&accentColor=red`：Basic 与 Advanced 之间切换后外框一致；测试连接、取消、保存始终显示在固定底栏；高级页四组字段及并排连接参数均完整可见；SSH 密钥切换后私钥文件选择器、说明入口、可选口令与凭据存储仍在原流程中。
- V3.4 Web 矩阵实际设置并回读 `860×640`、`988×756` 和 `1440×920` 视口；视口、DPR、主题、语言、密度、字体及横向溢出检查通过。另有高级配置和编辑外观独立截图，见 `screenshots/baseline-v34/manual/`。
- `npm --prefix frontend run type-check`、`git diff --check`：通过；完整测试结果汇总在 P7。
- P2 总状态：**Web Mock 布局通过**；文件选择器的原生系统对话框及真实背景图片资源行为未由浏览器验证。

## P3 安全、连接及文件弹窗

- `ConfirmDialog`、服务器分组/删除、工作区断开、文件删除及安装更新确认使用紧凑确认外观；普通操作使用中性主按钮，删除仍保留危险色。
- 首次 Host Key 核验、Host Key 变化和凭据查看使用 security 外观；Host Key 变化页保留新旧指纹、风险说明和明确拒绝动作。危险弹窗的焦点仍默认落在安全取消动作，REKA Dialog 行为不变。
- 活动传输断开时，确认区展示活动任务数量；确认按钮初始禁用，勾选理解提示后才允许继续。SFTP 删除确认明确显示完整远端路径。
- Web Mock 检查 Host Key 首次/变化、普通断开、活动传输断开、服务器删除、文件删除、分组管理及凭据查看。凭据查看 Mock 始终返回策略拒绝，因此只核对拒绝路径不泄露秘密；未验证二级密码成功查看。
- 分组弹窗第一次截图发现表单内容底部被裁切，已调整 form 变体的内容留白特异性；复查后输入框、新建按钮和底部留白完整可见。
- `npm --prefix frontend run type-check`、`git diff --check`：通过；完整测试结果汇总在 P7。未在原生桌面端或真实 SSH/SFTP 下验证。
- P3 总状态：**Web Mock 可见流程通过**；凭据查看成功路径和原生/真实后端状态未验证。

## P4 空态、服务器资源树与工作区标签

- 零服务器状态的标题收敛为 17px；唯一“添加服务器”按钮调整为 34px 高并使用中性主动作。无会话状态保留纯文字和准确快捷键，没有装饰图标或连接表单。
- 资源树保留唯一服务器列表入口；服务器未连接时显示中性状态点，行悬停仅使用背景色。组折叠和工作区关闭图标统一为 16px；`ShellIcon` 继续委托 `BaseIcon`，共享 viewBox 与描边，不重复实现 SVG 路径。
- Web Mock：零服务器页仅显示标题、说明和一个添加按钮；已有服务器但无会话显示文字和快捷键。依次打开 Web-01、DB-01 后可切换标签；切回 Web-01 时先前终端输出仍在。逐个关闭两个 Mock 工作区均保留断开确认，最后一个关闭后回到“暂无打开的会话”，服务器仍在资源树。
- `npm --prefix frontend run type-check`、`git diff --check`：通过；完整测试结果汇总在 P7。键盘树导航和原生窗口行为未单独验证。
- P4 总状态：**当前 Web Mock 流程通过**；原生桌面和键盘专项待后续验证。

## P5 SFTP、监控、设置及特殊区域

- Light + compact + red accent 的 SFTP 列表保留远端路径栏、面包屑、文件元数据和工具栏；长文件名在列表内收敛，软链目录仍按路径打开。Mock 页中路径为 `/opt/app`，打开软链后切到 `/opt/app/shared-current-release`。
- 分页 Mock 从第 1 页 001–010 切换到第 2 页 011–020，页面计数与首/下一页启用状态同步。
- 全局传输抽屉显示运行中、失败、完成、取消四种任务；展开后折叠，再打开第二个工作区，运行任务仍显示 72% 并按真实连接映射为 Web-01 / DB-01。此为静态 Mock 快照，不能证明实际字节传输继续。
- Monitor 检查 Light + Purple 的 stale 趋势、Dark + Red 的 error，以及 unsupported。陈旧值有明显状态、历史线变淡；错误/不支持时显示“暂不可用/不支持”且不显示趋势图。刷新按钮可操作，但 stale fixture 固定返回 stale，因此不能证明成功采集恢复。
- 专注模式只收起资源区与 Inspector，终端文本及其标签保留；退出入口仍可见。全局状态栏和应用工具栏保持当前主题 Token。
- Settings 的 General / Files / Network / Appearance / Terminal 页面仍是独立设置页，无保存底栏。关闭连接确认开关后切换到 Files 再回 General，值保持已改状态；随后恢复原值。网络页展示本地/在线 IP 查询选择和数据库操作。外观页仍有主题、密度、侧栏宽度、六种 Toast 位置、六个预设 accent、自定义颜色、应用图标；终端页仍显示图片主题的 imageId 预览、文字/光标/选区颜色、适配、九宫格位置、图片透明度、遮罩颜色/强度、模糊、字体、字号、行高、光标、闪烁、滚动行数及选中复制。
- Web `page=files-long-name` 的文本文件查看器显示正确的 `/opt/app/package.json` 路径，但 Mock 未实现 `sftp_read_text`，返回重试错误；没有据此宣称真实内容可读取。上传选择器和真实下载也未由浏览器 Mock 证明。
- P5 总状态：**页面外观与 Mock 可交互状态通过**；真实 SFTP 读写/上传/下载、监控采集恢复、重启后设置恢复尚未验证。

## P6 主题与历史数据兼容

- 在 Web Mock 中检查应用 Light/Dark/System、Standard/Compact、侧栏宽度、六种 Toast 位置、六个 accent 预设、自定义颜色和应用图标；主题变化即时反映在外层界面，自定义颜色控件能显示当前值。
- 终端设置页检查文字/光标/选区、字体、字号、行高、光标样式与闪烁、滚动缓冲、图片模式、适配、位置、透明度、遮罩、模糊等配置控件；图片预览和带图片背景的 Mock 终端可见。
- 编辑 Web-01 的 Appearance 页检查标签颜色、继承全局终端外观及切换服务器级覆盖；离开前明确丢弃了临时改动，再检查 DB-01 仍为继承全局外观。该操作只证明 Web 控件和未保存草稿行为，不证明 Core 持久化或重启后恢复。
- **P6 Web 阶段通过**；旧版数据库迁移、真实 Core revision、图片资源重启恢复、Mac/Windows 历史配置兼容未验证。

## P7 视觉矩阵、行为回归与人工复核

- 新建 `screenshots/baseline-v34/`，不覆盖 V3.3 与 MauLink Blue 原有基线。复用现有 V3.3 确定性路由和矩阵条件，完成 9 组、159 个页面状态：Light/Dark/System、中文/英文、Standard/Compact、blue/red/custom accent，视口 `860×640`、`988×756`、`1440×920`。
- 每页均独立加载两次；实际 CSS 视口、DPR、主题、语言、密度、字体加载、横向溢出检查通过，两个截图 SHA256 一致。9 组 manifest 与 159 张 JPEG 保存在 `screenshots/baseline-v34/full-matrix/`。
- 补充 Basic 窄屏、Advanced、SSH key、编辑 Appearance、主机地址校验错误 5 个 Web 状态；各状态二次加载哈希一致，记录见 `screenshots/baseline-v34/manual/` 和 `screenshots/baseline-v34/custom-review/`。普通/活动传输断开、服务器/分组/文件删除、首次/变化 Host Key、零服务器和无会话已有独立矩阵页面。测试连接等待态由延迟 Promise 单测验证，Mock 立即响应，未制作伪造的等待态截图。
- 人工复核全部 9 组联系表，重点检查服务器表单/危险确认/Host Key/设置/终端/文件/监控的密度与可读性；另查看 Advanced、私钥选择、服务外观和校验错误原始截图。未发现截断、重叠或横向溢出。此 V3.4 基线为本轮审查结果，不代表用户已将其冻结为未来设计标准。
- 自动化结果：`npm --prefix frontend run type-check` 通过；`npm --prefix frontend run test` 通过（32 个测试文件、322 个测试）；`git diff --check` 通过。新增延迟 draft connection test 断言测试按钮忙状态和保存按钮禁用；弹窗尺寸测试同步更新为紧凑变体。
- P7 总状态：**Web、自动化及候选基线检查通过**。真实原生焦点/窗口行为仍属于 P8。

## P8 桌面发布门禁

- 本轮完成前端 Vite 生产构建和 Tauri 桌面应用重建；结果与具体包路径记录在最终交付说明。
- 未执行 Windows 原生构建/运行，未连接真实 SSH/SFTP/Monitor 主机；未验证活动任务期间升级、终端输入法/Ctrl+C、原生文件选择、历史配置迁移与重启后图片/密钥恢复。Web Mock、单元测试和本机打包均不替代这些平台及后端验收。
- P8 状态：**当前环境可执行项已完成；跨平台和真实服务发布门禁待具备对应环境后验证**。

## 差异与风险

- 交付包中 P0–P8 阶段定义与现有 V3.3 源码兼容；没有要求从头重做工作区或更改 IPC。
- 中性主动作由独立 `neutralPrimary` 变体承载，不覆盖 `primary` 的 accent 行为。
- `BaseDialog` 仍保留 Reka Dialog/AlertDialog、焦点捕获、Esc、关闭及返回焦点。视觉迁移不得替换这些原语。
- Browser Mock 可以验证布局与前端交互；原生窗口、Windows 平台、真实 SSH/SFTP 和旧版配置升级需单独记录，不从浏览器通过推断。
