# MauLink 前端逐页改造验收记录

检查日期：2026-10-01（Asia/Shanghai）。当前实现：Vue 正式 frontend；旧入口由切换前 Git 提交 f75d21a 保留。

> Phase 10 的历史截图、capture manifest 和画廊已从当前仓库清理；本页文字保留当时的 QA 结论。当前视觉回归基线位于 `docs/refactor/screenshots/color-v1-review/`。

**Browser visual evidence: PASS（Phase 10）**。

**完整产品 / 双平台 release QA：final result: blocked**。该状态包含已有后端能力、Phase 6 行为和平台缺口，不再以“截图未落盘”为原因。用户取消后续 Native 手动门禁，阶段状态按报告的授权范围判断；未实测项不会被改写为 Native PASS。

2026-09-30 的旧前端历史检查保存在 Git 历史；本文件当前逐页结果针对 Vue frontend，不把旧版动态 SSH / 不同 DPR 的截图当新视觉基线。

## 对照材料与证据

- 原型：本机 `UI/MauLink_prototype_local.html`，SHA256 `fb9e3ad1675fdea4e67d0f1f90b80c160404be02c7ac4faae05bc002fbe02063`；原型文件不纳入 Git。
- 原型参考：曾使用临时冻结副本，保留 layout/CSS；固定 demo clock/random、停止 intervals、隔离偏好、对齐服务器数量/表单默认值/文件清单/传输有无。变换摘要与源 hash记录在 Phase 10 报告中；对应截图归档已清理。不把演示内容当真实 Core。
- 实现：`?harness=visual&page=…&theme=light|dark&locale=zh-CN|en`，生产组件 + Typed Mock IPC。`page` 与确定性 UI 操作 recipe 见 [cases.json](frontend/visual/cases.json)；URL 固定 fixture，脚本点击真实可见控件进入目标状态。
- 同一 Browser，CSS **1440×920、860×640**，DPR **1**，Light/Dark、zh-CN/English；每张 JPEG 真实像素尺寸与 CSS×DPR 一致。系统字体、Asia/Shanghai 文件日期，不混用历史 DPR=2 Native 图片。
- 实现图曾覆盖 23 场景 × 8 条件（**184 张**），原型参考图覆盖 20 场景 × 8 条件（**160 张**）；两次独立装载 byte-identical。对应历史图像与 manifest 已清理。
- 历史双列画廊和 summary 已随截图归档清理；当前基线重跑与校验方式见 [视觉回归说明](frontend/visual/README.md)。
- 默认回归保留旧基线：缺失/变化/重复不等均失败并输出候选，仅审阅后显式 update。23 张中文浅色宽屏及偏好隔离后的八种终端设置条件已另行通过默认 baseline 比较。
- 原型 `.app` min-width=960px，在 CSS 860px 下出现横向溢出；如实保留原始布局。新实现所有 184 个状态无横向溢出。两者相同 CSS 条件可做结构对照，不要求不同产品能力下的像素相等。

## 逐页状态

“证据 PASS”仅表示页面状态、固定条件、落盘与重复验证完成；功能 / Core / 原生平台结果独立记录。

| Page | 页面 | 当前状态 | 证据与实际差异 / 未完成项 |
| --- | --- | --- | --- |
| 0 | Shell 基线 | 证据 PASS；精确 fidelity 部分 | empty / servers 双主题双语双视口；token、留白与圆角可对照，不宣称全部 hover/focus 或逐像素一致。 |
| 1 | Application Shell | 证据 PASS；Native 部分 | Sidebar / Topbar / Statusbar / user groups 固定。Browser 为真实 macOS traffic-light 留白，原型是假交通灯；原生窗口四尺寸、minimize/restore 属 Phase 11。 |
| 2 | Welcome / Empty Home | 证据 PASS | empty 8 条件。新增服务器、About 等生产入口沿用；未伪造 Core 中不存在的 demo 导入或访问外链。 |
| 3 | Server Home / Cards | 证据 PASS | servers 8 条件，三个固定 profile、两个 group。用户分组名保留原文；卡片菜单与留白存在已有产品差异。CRUD 功能见 Phase 4。 |
| 4 | Add Server | 证据 PASS | add-server 8 条件；固定默认字段，不保存凭据。生产表单/认证/token 等功能见 Phase 4；原型 segmented controls 与实现呈现不同。 |
| 5 | Edit / Delete Server | 证据 PASS | edit-server / delete-server 各 8 条件。Delete 只打开确认，不执行真实删除；revision/credential 行为见 Phase 4。 |
| 6 | Connection Security / Errors | 证据 PASS | 首次/变化 Host Key、authentication、connection-error 各 8 条件。实现保留安全语义，原型是简化示意；真实连接见 Phase 5 / 用户 Phase 9。 |
| 7 | Workspace / Terminal | 证据 PASS；完整行为仍有缺口 | terminal 8 条件，经生产 xterm 和 Channel 输出固定 UTF-8/ANSI。真实 SSH/输入用户 Phase 9 已确认；Phase 6 Ctrl+C/vim/突发输出等缺口不因截图消失。 |
| 8 | Terminal Focus Mode | 证据 PASS | focus 8 条件，实际 UI 点击进入，宽度变化有固定输出证据；原型专注层与真实 xterm 结构不同，完整快捷键行为参见 Phase 6 / 9。 |
| 9 | Files / SFTP | 证据 PASS | files / transfer 各 8 条件，同一 POSIX path 和共享文件 fixture；固定传输进度。真实 SFTP / task 语义见 Phase 7，不用静态进度代替 integration。 |
| 10 | Remote File View / Edit / Delete | Delete 证据 PASS；View/Edit BLOCKED | file-delete 8 条件；View/Edit 因既有后端缺少文本读写 IPC 持续禁用。这是既有能力差距，不伪造文本读取/保存。 |
| 11 | Monitor | 证据 PASS；真实 Linux 成功态未现场验证 | monitor / monitor-unavailable 各 8 条件。共享契约质量与 history；原型 process/latency 等缺后端的数据不添加。Core 及 concurrency 见 Phase 8。 |
| 12 | Settings / General | 证据 PASS；原型能力不同 | settings-general 8 条件；只呈现 Core 支持的断开确认。没有伪造恢复会话/自动更新。用户 Phase 9 已确认保存与重启。 |
| 13 | Settings / Appearance | 证据 PASS | settings-appearance 8 条件；保存型 Dialog 对照原型全页即时设置。Light/Dark 全矩阵；System 用户 Phase 9 实测，操作系统实际切换未现场验证。 |
| 14 | Settings / Terminal | 证据 PASS | settings-terminal 8 条件，字号/光标/copy/font/scrollback 与契约一致。无伪底部输入模式；活动应用及重启持久化用户 Phase 9 确认。 |
| 15 | Settings / Language | 证据 PASS | settings-language 和所有其他场景均 zh-CN/en；参数与单复数有自动化保护，远端内容不翻译。 |
| 16 | Palette / Context Menu / Toast | 证据 PASS | palette / context-menu / toast 各 8 条件。后两项及 Monitor unavailable 无对应原型固定 hash，但有实现基线；键盘及 restore 见 Phase 9。 |
| 17 | Dark Theme Regression | 证据 PASS | 23 个实现状态 × 双语 × 双视口 = 92 张 Dark；全部独立重复一致，无横向溢出。完整 WCAG / 全 hover 状态未另立审计。 |
| 18 | Responsive / macOS / Windows | Browser 四视口 PASS；完整平台 BLOCKED | 860×640、1440×920 全量固定场景；1080×760、1920×1080 各 8 场景独立重复截图；同一活动终端 resize / 专注退出保持实例、输出及焦点。见 Phase 11 报告。原生最小化/恢复及 Windows WebView2 / 凭据 / 路径 / installer 未实测；按用户 Browser 门禁继续。 |
| 19 | Legacy CSS Cleanup | PASS | Phase 12 将 Vue 工程切换为 frontend；旧 .mjs / legacy CSS / vendor 从活动入口移除，Git f75d21a 可恢复。 |
| 20 | Final Screenshot QA | Browser 证据 PASS；完整产品部分 | 344 张可重复图片、同条件参考、hash/真实像素校验与画廊落盘。没有再用历史不同 viewport/DPR、真实 SSH 动态内容做严格基线；平台与能力问题单独保留。 |

## 视觉对照结论

- **字体 / 排版：**同环境的 baseline 可做回归，双语无页面横向溢出；终端使用 xterm 与 monospace。原型与产品控件类型、文案及字体 stack 有已知差异，不以截图一致性测试宣称相同排版。
- **间距 / 布局：**宽屏双列 Server Card、窄屏单列、Sidebar、Files / Transfer 分区、Dialogs 均有固定证据。Server 页内边距、Settings Dialog 对原型全页、Monitor quality/history 对原型 demo panel 的差异明确保留。
- **颜色 / token：**Light/Dark 贯穿全部实现状态；原型 860px 自身溢出有明确记录。未通过修改原型 min-width 或生产色值制造假匹配。
- **图标：**使用项目现有图标系统；文件类型符号、真实交通灯留白、更多操作按钮与原型存在已知差异，无新增图片资源。
- **文案 / 数据：**模块 catalog、用户名称与远端路径分开；Mock 与真实 Core 证据分开。参考 demo metrics 与真实契约范围不同，不能以 prototype process table 假装后端实现。

## 未解决项

- **[P1 / 既有能力差距]** 远程文件文本 View/Edit 无对应 Core IPC。本阶段没有扩大后端范围、伪造 preview/save 或将其记为完成。
- **[P2]** Phase 6 完整终端行为验收缺口保留；此前用户允许先推送进入后续阶段，未将 Phase 6 改为全面 PASS。
- **[P2]** Windows / 原生四尺寸、minimize/restore、真实 Linux 成功指标等完整 release QA 未全部现场完成；用户 Browser 门禁不等于 Native PASS。
- 正式切换和旧入口清理见 Phase 12 报告；既有 Core/平台能力缺口仍保留。

“实现截图未落盘 / 无固定视觉 fixture / 不同 CSS 视口导致不可对照”的旧阻塞已解决。具体自动化、Browser 过程、限制和退出范围见 [Phase 10 报告](docs/refactor/frontend-v2-phase-10.md)。

## MauLink V3.3 P7 视觉回归（2026-10-15）

**Reference:** `docs/redesign-v33/screenshots/prototype-reference/idle-dark.png` and `sftp-dark.png` (988×756). The interactive prototype remained available at `http://127.0.0.1:8888/`.

**Implementation:** `docs/redesign-v33/screenshots/candidate/dark-zh-standard-988x756/servers-dark-zh-CN-988x756.jpg` and `files-dark-zh-CN-988x756.jpg`, plus eight other viewport/theme/density/accent conditions. The reviewed reference copy is `docs/redesign-v33/screenshots/baseline-v33/`.

Both source and implementation reference images use CSS **988×756**, DPR **1**, dark theme, and Simplified Chinese. Full-scene implementation evidence has **159 screenshots across 9 conditions**; the source prototype supplies three static reference images, so this report does not claim a one-to-one 159-screen prototype capture. The `servers` and `files` comparisons cover the closest static reference states. The prototype includes demo transfer counts and workspace data; the implementation uses typed mock fixture states and preserves actual product capabilities. Prototype window controls are illustrative and are not copied into the Tauri shell.

**Findings and fixes:**

- Browser evidence showed the normal terminal shell had a `display:block` override that collapsed `.shell-main-content` and left the terminal stack at zero height. Removing the override restored a 530px terminal area at 988×756 and made the terminal background image visible.
- The compact density rule overrode the Files two-row layout. A compact Files override restored the file table to 494.5px at 988×756. The screenshot recipe now fails if the file list is below 100px.
- The transfer screenshot recipe now opens the global transfer drawer. Settings Files no longer waits for a remote-file table. English accessible labels match the actual controls.
- Nine candidate contact sheets were inspected. No remaining blank terminal/file panes, horizontal overflow, or clipped content appeared in the reviewed conditions. Existing product/prototype differences are documented above and in `docs/redesign-v33/00-audit.md`.

The V3.3 verifier passed all 159 candidate images and confirmed candidate/baseline equality. The historical 82-image verifier also passed unchanged. The 159-page PDF was rendered and pages 1, 80, and 159 were inspected; each keeps the full screenshot and readable condition/page labels.

**Final result: PASSED (Web visual scope).** Real SSH/SFTP, Core persistence across app restart, macOS native-window behavior, and Windows packaging/runtime are outside this browser evidence and remain in P8.

## P8 桌面打包复核（2026-10-15）

macOS 15.7.9 / Apple Silicon 已生成 `0.1.1-rc.8` arm64 DMG。`hdiutil verify` 与挂载后的 bundle/code-signature 检查通过；当前是 ad-hoc 签名且无 Apple 公证凭据。`open -n` 返回 `kLSNoExecutableErr (-10827)`，CUA 未获准控制此 app，因此原生窗口启动仍未验收。Windows MSI、macOS x64、真实 SSH/SFTP 与 Core 重启持久化未在当前主机验证。完整记录见 `docs/redesign-v33/00-audit.md`。

## 外观与显示页面改版（2026-10-19）

**结果：PASS（Web 视觉范围；通知位置单选项的浏览器自动化点击未完成）。**

- 参考：本任务附带的第一张“外观与显示”设计图。
- 实现：Codex 内置浏览器，本地 visual harness；视口 1368 × 906。CUA 截图在本轮以内联图像捕获并进行对照；该接口未提供仓库可用的截图文件路径。
- 页面改为标题与恢复默认操作区、左侧分组设置、右侧实时预览；设置导航补充语义图标。
- 浏览器复核主题浅/深切换与右侧预览同步、紧凑密度与预览摘要同步，以及恢复默认值。滚动后确认文件列表和通知位置区域排版正常。
- `npm run type-check`、`npm run build`、`git diff --check` 均通过。Vite 构建有主 JS chunk 超过 500 kB 的提示，不影响构建完成。
- 未运行测试套件。通知位置选项未能通过浏览器辅助功能自动点击，因此不记录为交互 PASS；1368 × 906 首屏低于设计图高度，底部通知区通过滚动复核。

## 外观页样式复原（2026-10-19）

**最终结果：PASS（Web 视觉范围）。**

- 参考：本轮用户附带的终端设置页面截图。
- Codex 内置浏览器在 1368 × 906 视口复核外观页：共享设置标题栏保留单行标题与说明，设置内容以居中单列排列；与终端设置页的间距、宽度和基础控件风格一致。
- 移除了额外页面标题区、右侧模拟预览和恢复默认按钮；现有主题、密度、侧栏、文件列表、通知、主题色和应用图标控件仍保留。滚动检查确认后续控件未被裁切。
- 仅验证本地 visual harness 的页面结构和视觉排版；未改动设置行为，也未运行测试套件。

## 文件列表显示选项改为下拉（2026-10-19）

**结果：PASS（Browser UI 范围）。**

- Codex 内置浏览器打开本地 Settings harness，外观页中“显示大小列 / 显示文件大小 / 显示文件夹大小”均以与上方主题、密度相同的下拉控件展示，选项为“显示 / 隐藏”。
- 交互复核：将“显示大小列”切换为“隐藏”后，依赖的文件及文件夹大小选项收起；恢复为“显示”后两个选项恢复，初始值也恢复。
- Legacy SettingsDialog 中相同设置同步使用下拉控件。只验证本地内存 Mock，不代表真实 Core 重启持久化验证；未运行测试套件。
- `npm run type-check`、`npm run build` 和 `git diff --check` 均通过。Vite 仍提示主 JS chunk 超过 500 kB。

## 外观页应用图标与主题色顺序调整（2026-10-19）

**结果：PASS（Browser UI 范围）。**

- Codex 内置浏览器打开本地 Settings harness，确认应用图标样式位于外观页顶部，主题色选项紧随其下，其余控件顺序保持不变。
- 应用图标设置上方的分隔线已移除；只验证本地 Mock 页面结构与视觉排版，未运行测试套件。
- `npm run type-check`、`npm run build` 和 `git diff --check` 通过。Vite 仍提示主 JS chunk 超过 500 kB。

## 通用设置更新区布局调整（2026-10-19）

**结果：PASS（Browser UI 范围）。**

- Codex 内置浏览器复核通用设置页：连接行为和界面语言区块之间的横向分隔线已移除，区块间距保留。
- 更新卡片将版本与安装渠道整理为紧凑元信息；更新渠道、说明及当前主要操作在宽屏下同一行呈现，窄屏断点下改为纵向布局。
- 本地 Settings harness 仅提供 Mock 设置与更新状态；本次仅验证静态页面结构和当前空闲状态的视觉布局，不代表真实更新服务行为验证。未运行测试套件。
- `npm --prefix frontend run type-check`、`npm --prefix frontend run build`、`git diff --check` 均通过。Vite 仍提示主 JS chunk 超过 500 kB。
