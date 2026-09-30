# MauLink 前端逐页改造验收记录

**检查日期：2026-09-30**
final result: blocked

## 对照材料与截图证据

- 视觉基准：本机交互原型 [MauLink_prototype_local.html](/Users/mauriceliang/Documents/code/MauLink/UI/MauLink_prototype_local.html)，文件页地址 `http://127.0.0.1:8080/MauLink_prototype_local.html#view=files&select=web-01`。改造计划：`/Users/mauriceliang/Downloads/MauLink_UI逐页改造计划_Codex执行版_v1.1.md`。
- 实现：`frontend/index.html`、`frontend/styles.css`、`frontend/src/main.mjs`、`frontend/src/i18n.mjs`、`frontend/src/command-palette.mjs`、`frontend/src/terminal-preferences.mjs`。
- 实现截图路径：**未落盘**。CUA 截图只在本次工具输出中显示，未生成可复用的项目内 PNG 文件。
- 本轮 Settings / Terminal 与 Add Server 在隔离 Tauri 构建中复核。QA 窗口配置为 **906×756 logical px**，截图输出为 **1812×1512 device px**（DPR 2）；原型截图输出为 **906×756 px**，其 CSS 视口/DPR 未通过浏览器接口读取。视口证据不支持像素级差分。
- 此前 Files 页检查使用浅色主题：原型使用虚构的 Web-01、示例文件和三条演示传输；产品使用隔离配置 `io.maulink.uiqa.20260929`、本机回环 SSH/SFTP（`127.0.0.1:49224`）、测试文件和实际传输记录。内容、传输数和视口不一致，截图只可用于粗看结构与 token，不能作为严格视觉验收证据。本轮 Add Server 与 Settings 检查未连接服务器。
- 本轮 Settings / Terminal 原型与运行态：原型当前可见 14 px、方块光标、“选中即复制”关闭，以及“底部输入栏 / 终端内联输入”选择卡。最新应用在空隔离 profile 中确认 14 px、方块光标、复制关闭；字体 `monospace` 和 10000 行滚动缓冲保留在折叠的高级区。原型未显示字体或滚动缓冲控件。应用没有独立的底部命令输入模式，未将交互式 xterm 替换为输入框。
- 本轮 Add Server：浅色原型与最新 Tauri QA 构建的弹窗外框约为同一尺寸（原型约 526×557 px，产品 CSS 目标 526×556 px）；字段顺序、认证切换、凭据说明、高级连接选项和按钮层级一致。产品中验证了密码/SSH 密钥切换与高级区展开，表单未提交；原型按钮的 CUA 点击未改变认证/高级状态，因此这些交互只以原型静态默认态和产品真实状态作视觉对照。
- 本轮 Empty Home：空隔离 profile 的首页在浅色、深色都已打开查看；中央图标、标题、说明、主按钮和侧栏无服务器状态可见。为避免改变服务器资料，未点击原型的 Load demo servers；Learn about MauLink 外链也未打开。
- 局部截图：未单独裁切。当前截图接口只返回整窗图；密集文件表格的字号、边框与行距还需在同尺寸截图或局部对照中复核。

## 已完成的修正与运行态验证

- 先前运行态发现文件修改时间占用过宽、无速率数据时显示孤立的“—”。已将时间格式改为紧凑的 `Sep 29 22:51`，并在没有有效速率/剩余时间时隐藏该元信息。最新 Tauri 构建的真实 SFTP 下载完成后，传输卡只显示文件名与完成状态，未出现孤立占位符。
- 文件页在隔离回环服务器上完成目录浏览、分页加载、创建文件夹、上传、重命名、下载及刷新验证；操作均限于临时 fixture。删除确认流程尚未执行，等待单独的操作时确认。
- Page 16 命令面板已在 Tauri 中验证打开、搜索、方向键跳过禁用项、Enter 执行动作、Escape 关闭；文件行菜单中下载、复制路径、重命名和删除可用，远程查看/编辑因后端无对应接口而禁用。
- Settings 的外观、通用、终端、语言页曾在隔离应用中查看；本轮在最新 CSS 构建中检查了终端设置浅/深主题，修正深色下设置标签的低对比度，并恢复浅色。此前 English 单终端复数检查结果仍有效。主题与语言变化均限于隔离 QA profile。
- Monitor 在本机 fixture 下正确显示指标暂不可用状态；没有 Linux 服务器指标可供验证，没有伪造监控数据。

## 逐页状态

| Page | 页面 | 状态 | 证据或未完成项 |
| --- | --- | --- | --- |
| 0 | Shell 基线 | 部分 | token 与 CSS legacy 清单已记录；缺少同尺寸落盘截图和全部 hover/focus 对照。 |
| 1 | Application Shell | 部分 | 已检查实际 macOS 应用框架和原生交通灯位置；完整尺寸对照、截图归档未完成。 |
| 2 | Welcome / Empty Home | 部分 | 空隔离 profile 的浅色与深色首页已运行态查看，主要层级和侧栏状态可见；未改变 profile 加载 demo servers，也未打开 Learn about MauLink 外链，因此对应完整点击路径未验收。 |
| 3 | Server Home / Cards | 未通过 | 当前实机 profile 与原型演示服务器数据不一致，分组/卡片全状态未逐项核验。 |
| 4 | Add Server | 部分 | 弹窗宽高已收敛到约 526×556 px，浅色截图与原型结构对齐；密码/SSH 密钥切换和高级区展开已在隔离运行态验证。表单验证、测试连接、保存 Toast 未验收。 |
| 5 | Edit / Delete Server | 未通过 | 产品资料编辑与删除完整运行路径未验收。 |
| 6 | Connection Security / Errors | 未通过 | Host Key、凭据挑战、Host Key 变化及连接错误路径未全部重跑。 |
| 7 | Workspace / Terminal | 部分 | 已进入真实回环 workspace；终端输入、生命周期及原型逐状态对照未完成。 |
| 8 | Terminal Focus Mode | 未通过 | 焦点模式的进入、退出和键盘行为未验收。 |
| 9 | Files / SFTP | 部分 | 回环 SFTP 目录、分页、创建目录、上传、重命名、下载和完成传输卡已实测；删除动作待确认。 |
| 10 | Remote File View / Edit / Delete | 未通过 | 查看和编辑目前因缺少远程文本读写 IPC 而禁用；删除确认流程未执行。不能以模拟内容代替真实文件。 |
| 11 | Monitor | 部分 | 不可用状态运行正常；没有可用 Linux 指标源验证成功数据态。 |
| 12 | Settings / General | 部分 | 设置页已查看；完整控件与持久化逐项复核未完成。 |
| 13 | Settings / Appearance | 部分 | 隔离 profile 已切换暗色并恢复浅色；其余页面的深色回归未逐页完成。 |
| 14 | Settings / Terminal | 部分 | 顶层字号预设、光标分段选项、选中即复制已运行态复核；字体/自定义字号/滚动缓冲收在折叠高级区，深色标签对比度已修正。没有服务器连接，因此设置应用到活动 xterm 的效果未验证。原型输入方式卡片不适用于当前真实交互式 xterm，没有伪造底部命令输入。 |
| 15 | Settings / Language | 部分 | 中英文切换、翻译和单终端复数已实测；完整文案矩阵尚未验证。 |
| 16 | Palette / Context Menu / Toast | 部分 | 命令面板键盘路径、文件菜单和若干文件操作提示已测；服务器菜单与所有 Toast 变体未验收。 |
| 17 | Dark Theme Regression | 未通过 | 原型深色入口已查看，产品只在有限页面切换过主题；未覆盖全部页面与弹窗。 |
| 18 | Responsive / macOS / Windows | 未通过 | CSS 断点已检查；计划的四种尺寸未全部在运行态捕获，Windows 环境不可用。 |
| 19 | Legacy CSS Cleanup | 未通过 | `frontend/styles.css` 仍有旧规则与 prototype-aligned 后置覆盖；尚未完成 selector 使用审计和逐页删除。 |
| 20 | Final Screenshot QA | 未通过 | 缺少相同 CSS 视口/状态/密度的落盘实现截图与最终逐屏比较。 |

## 必需视觉面的检查结论

- **字体与排版：**文件行、表格列和传输状态在运行态可读；原型与应用截图视口不同，没有足够证据比较字形宽度、字重、行高或截断差异。
- **间距与布局：**侧栏/工具栏尺寸 token 已按原型建立，文件浏览区与传输区的结构已运行。两张文件页画面的视口和示例内容不同，因此比例、行距及边距尚未通过同尺寸核验。
- **颜色与 token：**浅色紫色强调和灰白表面已运行；深色产品回归未完成，当前没有同密度像素采样。
- **图像与图标：**已检查的 shell 与文件页继续使用项目现有图标系统；本次目标区域没有需要新增的照片或插画。原型与实现的图标细节仍待局部同尺寸对照。
- **文案与内容：**中英文 UI 翻译包含单复数运行态检查；SFTP 时间格式与无速率传输卡已修正。动态服务器、路径、文件列表因原型为演示资料而不要求逐字相同。

## 阻塞问题与下一步

- **[P1] 文件查看/编辑能力缺失。**原型提供远程文件查看/编辑入口，应用因现有后端未提供文本读取/写回接口而禁用。保持现有 IPC 契约时不能实现真实编辑，也不能伪造预览内容；需另立后端能力范围后才能补齐该页。
- **[P2] 视觉验收证据不满足对照条件。**本轮 CUA 没有提供将截图保存为指定路径的接口；虽然 Add Server 与 Settings / Terminal 已在相同配置逻辑视口检查，但截图没有落盘，也未做像素差分。仍需归档同主题/状态的全景与文件表格局部截图。
- **[P2] 页面覆盖与平台验收仍不完整。**余下页面交互、深色全量回归和四种 macOS 窗口尺寸未全部完成；Windows 运行验收依赖可用的 Windows 环境。debug/release bundle ID 冲突已通过 `io.maulink.uiqa.20260930modal` 隔离构建绕开；`cargo tauri` 子命令仍不可用，但 Cargo 离线构建与实际 QA 窗口运行已验证。
- **[P2] CSS 仍有旧规则叠加。**完成页面视觉验收后，按组件实际使用关系逐步清理旧绿色主题与重复覆盖，避免一次性删除仍被页面使用的规则。

## 自动化与构建检查

截至 2026-09-30：`node --check`（`main.mjs`、`i18n.mjs`、`command-palette.mjs`、`terminal-preferences.mjs`）通过；`node --test frontend/tests/*.test.mjs` 为 24 项通过、0 失败；`git diff --check` 通过；本轮正式 Cargo 离线构建与两个唯一 bundle ID 的隔离 QA 构建均通过。最新 QA 窗口实际显示了调整后的 Add Server 弹窗、Terminal 设置页及空首页浅/深主题。

本文件保持 **final result: blocked**，直到截图对照门禁、待验收页面和已知能力限制得到解决；不将交互原型的演示数据算作产品运行证据。
