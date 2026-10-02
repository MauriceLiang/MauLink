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
