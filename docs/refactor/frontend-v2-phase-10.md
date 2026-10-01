# Phase 10 验收报告：Browser UI Harness 与视觉回归

检查日期：2026-10-01（Asia/Shanghai）。前置：Phase 9 `74be8b3` 已通过用户独立 macOS release 实测并推送 main。

## 1. 完成项

- 新增 DEV-only `?harness=visual&page=…&theme=light|dark&locale=zh-CN|en` test route，固定 Typed Mock IPC；复用生产 AppShell、Dialogs、xterm、Files、Transfers、Monitor、Settings、Palette 与 Context Menu，不复制页面布局。
- `visual/cases.json` 为 23 个稳定场景及真实 UI 操作 recipe。URL 固定数据，脚本通过页面可见控件进入对应页面；不会注入 DOM、伪造键盘事件或用真实 SSH 动态输出作视觉基线。
- 固定 server/connection/challenge/terminal/transfer ID、文件、时间、监控 snapshot/history、静态进度与终端 UTF-8/ANSI transcript。原生产 stores、Channel、output consumer 和 timer 仍正常工作，Mock 只返回固定 DTO。
- 主题、语言由 Mock SettingsRecord 经生产 SettingsService facade 应用。DEV 页面使用新内存 Storage 隔离 copy-on-select，不修改原有持久偏好；每次 reload 重置。所有不支持的 Mock 写操作显式失败，不降级到 Native IPC。
- 固定 CSS 1440×920 / 860×640、DPR=1，覆盖 Light/Dark × zh-CN/en；每次保存前读取实际视口、DPR、theme、lang、fonts 与横向溢出。仅 DEV 视觉样式停止时间动画与文本/终端光标，保留 focus ring；生产 CSS 不改变。
- CUA 自动生成 **184 张实现基线**与 **160 张冻结原型参考图**。每张均经两次独立装载比较，JPEG 字节完全一致；23 张中文浅色宽屏基线另经默认回归模式重新通过。
- 默认回归模式保留旧 baseline，缺失/变化/重复不等时输出 candidate/repeat 并报错。仅显式 `update:true` 更新基线；canonical manifest 只在整轮成功后发布，执行 checkpoint 分开写。八种终端设置条件在偏好隔离后重新匹配已有基线。
- 独立 Node 校验 manifest 数量、SHA256、重复 hash 与 JPEG 真实像素尺寸，生成双列对照画廊、summary。`design-qa.md` 的每页状态已重写，消除“实现截图未落盘”问题，保留实际能力/平台缺口。

## 2. 修改文件

```text
frontend-v2/src/main.ts
frontend-v2/src/harness/VisualHarness.vue
frontend-v2/src/harness/visual-fixtures.ts
frontend-v2/src/harness/monitor-fixtures.ts
frontend-v2/tests/visual.test.ts
frontend-v2/visual/{cases.json,files.json,capture.mjs,capture.test.mjs,prepare-prototype.mjs,verify.mjs,README.md}
frontend-v2/README.md
docs/refactor/frontend-v2-phase-10.md
docs/refactor/screenshots/phase-10/{implementation,reference}/*
docs/refactor/screenshots/phase-10/{summary.json,prototype-source.json,index.html}
design-qa.md
```

`monitorFixture` 仅增加默认保持现有行为的可选时间参数，旧 Harness 和 tests 继续通过。Rust Core、contracts、旧 frontend、Tauri 配置、依赖与 lockfile 均未修改。

## 3. 新增依赖

无。TypeScript 仍精确 `5.9.3`。截图使用已有 CUA Browser API；截图校验、baseline 保护、画廊和原型副本准备仅使用 Node built-ins。未新增第三方浏览器 driver，也未在 shell 中运行浏览器自动化。

## 4. 自动化检查

### Frontend

| 检查 | 结果 |
| --- | --- |
| npm run type-check | PASS |
| npm run test | PASS，11 files、125/125 tests（本阶段新增 10） |
| node --test frontend-v2/visual/capture.test.mjs | PASS，1；基线缺失/变化、重复变化与显式 update 的保护行为 |
| npm run build | PASS；126 modules，JS 507.59 kB / gzip 143.51 kB，CSS 29.39 kB |
| node frontend-v2/visual/verify.mjs | PASS；184 implementation + 160 reference，DPR=1、重复 byte-identical |
| 生产 bundle 排除 Visual/Settings DEV fixture | PASS；production JS/CSS hash 与 Phase 9 构建相同 |
| git diff --check | PASS |

新增 Vitest 覆盖：route 白名单与安全回退、不同 wall clock 下相等的 DTO、固定传输轮询、空首页、四种挑战/失败、真实 Channel 边界固定输出、内存偏好隔离、未知写操作拒绝。

当前 Vitest 的 jsdom Canvas/localStorage 提示、Vite native configLoader 提示与 >500 kB chunk 警告沿用既有状态；命令成功，未调高阈值掩盖警告。

### Rust

fmt / check / clippy / test：本阶段没有 Rust 或契约修改，未重复执行，不能记为本阶段 PASS。前置阶段证据仍见各阶段报告；本次 Browser Mock 不代替 Rust/SSH integration。

## 5. Desktop 实际验证

- 本阶段未新增启动 Desktop；新增代码全部为 DEV visual route，生产 build 不包含其 fixture/style。
- Phase 9 的独立 release 用户实测已通过。此处不将其重复计算为新的跨平台 QA。
- 按用户持续的 Browser 门禁，本阶段不要求新增 Native 手动验收。Windows、系统凭据、真实 picker、原生 minimize/restore 仍未在本阶段验证。

## 6. 视觉验收

| 条件 | 实现 | 原型参考 |
| --- | --- | --- |
| Light / Dark | 每种均覆盖 23 场景 × 双语 × 双视口 | 每种均覆盖 20 场景 × 双语 × 双视口 |
| zh-CN / English | 主要 UI 双语；远端名称保留固定原文 | 两种语言固定 hash |
| CSS 1440×920 | 92 张，无横向溢出 | 80 张，无横向溢出 |
| CSS 860×640 | 92 张，无横向溢出 | 80 张；原型 min-width=960px 导致横向溢出，记录原样 |
| device scale | DPR=1，JPEG 真实像素与 CSS 尺寸一致 | 同一 Browser / DPR / CSS 条件 |
| 重复加载 | 184/184 byte-identical | 160/160 byte-identical |

现场为 macOS、系统字体、Asia/Shanghai 文件日期格式，未依赖外部字体或网络 fixture。不同 Browser/字体/时区/DPR 需独立环境基线，不能直接混用。

原始原型 `UI/MauLink_prototype_local.html` 不修改；临时冻结副本固定 demo clock/random、停止 intervals、隔离原型偏好、对齐服务器数量/表单默认值/文件清单/传输有无。源 SHA256 `fb9e3ad1675fdea4e67d0f1f90b80c160404be02c7ac4faae05bc002fbe02063` 与变换记录落盘。原型源本机可用但未纳入 Git，其他环境需提供该版本才能重建参考图。

原型 hash 仅在 boot 读取；脚本必须 reload，四个 Settings heading 均检查，防止 `termSec` 路由误归为 Language。参考图不会被当作原始未冻结截图或真实 Core 数据。

对照画廊：`docs/refactor/screenshots/phase-10/index.html`；所有图片、URL、条件和 hash 均可独立核对。截图一致性检查在“同实现两次装载”之间执行；原型与实现用画廊审阅结构/能力差异，**不宣称二者像素相等**。

## 7. 与旧前端差异

- 新前端正式数据继续走 Typed IPC；视觉 route 仅显式 Mock。旧 frontend / 正式 Tauri 默认入口保持。
- 用户命名的 Production/Development 不随 UI locale 翻译；原型用内置环境分类文案。Shell 保留真实原生 traffic-light 留白，参考图为演示交通灯。
- 设置为保存型 Dialog，原型为全页即时切换；通用设置只呈现真实 Core 能力，不虚构恢复会话/自动更新流程。终端使用真实 xterm，没有原型伪底部输入模式。
- 身份挑战保留 trust-once、trust-and-save、变化时默认拒绝及独立核实；原型为简化演示。Monitor 呈现真实契约的 quality/history，未添加原型中无后端能力的 process/latency 假数据。
- 文件 View/Edit 保持因后端缺接口而禁用。Baseline 的 Delete 只打开确认，不执行真实删除。
- 页面留白、控件呈现与 icon 系统的已有差异在逐页 QA 记录；本阶段未为了参考截图重写生产 UI。

## 8. 当前阻塞

本阶段视觉证据门禁无阻塞。完整产品 QA 仍有：Phase 6 剩余行为验收、远程文本 View/Edit 能力缺失、Windows/完整原生平台验证，以及后续正式切换/legacy 清理。这些缺口不能由本阶段截图消除，见 `design-qa.md`。

## 9. 是否达到退出条件

**PASS（Phase 10 Browser visual scope，按用户门禁）**。稳定 fixture/test route、同条件图片落盘、独立重复一致、默认回归保护、完整 Light/Dark/双语/两视口矩阵、可执行原型对照与逐页记录均已完成。可以按授权提交推送并进入 Phase 11；不将其写成完整 Desktop integration PASS。

## 10. 推荐 Commit

`test(frontend-v2): add deterministic visual regression harness`
