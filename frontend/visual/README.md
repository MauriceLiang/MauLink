# Phase 10：固定视觉回归

## 当前配色参考

2026-10-01 的 MauLink Blue 配色已由用户在 app 中确认。当前界面参考位于 `docs/refactor/screenshots/color-v1-review/`：中文 Light / Dark 1440×920 各 23 个场景，英文 Light / Dark 860×640 各 6 个场景，共 58 个组合。使用下面的 `captureCases` 时将 `outputDir` 指向该目录，并选择已有的视口 / 语言 / 页面组合；默认模式仍要求截图与已确认参考一致。

Phase 10 / 11 / 12 截图保留为旧配色的历史证据。下文的 184 张实现基线与 160 张原型图，以及 `verify.mjs` 的校验范围属于历史记录，不代表新色板与旧截图像素相同。完整新色板矩阵尚未扩展至 184 个组合，详见 [颜色系统验收](../../docs/refactor/color-system-v1.md)。

入口：`http://127.0.0.1:1420/?harness=visual&page=servers&theme=light&locale=zh-CN`。

`cases.json` 为稳定 test route 清单：URL 固定 fixture，`steps` 固定真实 UI 操作顺序。例如 add-server URL 先加载固定首页，再由脚本点击“添加服务器”；不会使用 DOM 注入、假事件、原生 Tauri 或真实 SSH。它不是新的产品路由。普通手动打开时也可按这些步骤进入目标页面。

23 个场景包括空首页、有服务器首页、新增/编辑/删除、首次/变化 Host Key、认证、连接失败、终端、专注、文件/删除确认、传输、Monitor 成功/不可用、四类设置、Palette、Context Menu、Toast。数据来自 Typed Mock IPC，读写隔离于正式资料。DEV 页面以全新内存 Storage 隔离选中即复制偏好，不修改原有浏览器持久数据；每次 reload 重置。不支持的操作显式失败，不降级为 Native IPC。

固定条件：Light/Dark × zh-CN/en × CSS 1440×920/860×640，现场 DPR=1。截图保存前读取实际 CSS 尺寸、DPR、theme、lang、font loading 与横向溢出；不把 `viewport.set()` 的请求值当作实测值。当前环境为 macOS 浏览器，系统字体，无外部 font/network fixture。文件日期按浏览器所在 Asia/Shanghai 时区格式化；换字体、浏览器、DPR 或时区需单独建环境基线。

文件名：`page-theme-locale-WxH.jpg`。截图来自 Codex CUA 内置 Browser，JPEG 尺寸与 manifest 的 CSS×DPR 用独立 Node 脚本校验。应用中的 timer/实际状态模块仍正常运行，但 Mock 返回固定值；DEV 视觉样式仅关闭动画/闪烁/文本光标，不删除 focus ring，也不进入生产包。

## 重跑实现

先在终端运行 `npm --prefix frontend run dev`。截图只在 `cua_repl` 中通过受支持接口执行，无额外浏览器驱动/依赖。先读取 CUA 浏览器、viewport 文档；使用同一浏览器，保持当前截图 Tab 为该浏览器的活动 Tab。后建的原型/画廊 Tab 会成为活动 Tab，应关闭它后再对实现截图。

```js
const capture = await import('/Users/mauriceliang/Documents/code/MauLink/frontend/visual/capture.mjs');
// tab 为 cua.createBrowserTab 返回的页面；viewport 为 browser.capabilities.get('viewport')。
await capture.captureCases({ tab, viewport,
  outputDir:'/Users/mauriceliang/Documents/code/MauLink/docs/refactor/screenshots/color-v1-review',
  width:1440,height:920,theme:'light',locale:'zh-CN' });
```

默认模式需要已存在且相等的 baseline。每个页面独立 reload 两次；重复不等、基线缺失/变化、实际条件不符时立即失败，生成 candidate/repeat 图，保留基线。仅在审阅后显式传 `update:true` 建立/更新图片；不改阈值掩盖变化。执行中写独立隐藏 checkpoint，完整成功后才更新 canonical manifest。`pages:['servers']` 可用于聚焦检查，其 manifest 单独命名；完整证据校验使用全部页面。

为避免将新图片静默当作通过，本阶段首次创建完整基线时显式使用 update，随后 23 个中文浅色宽屏页面已用默认模式通过回归。后续源码改变后的 candidate 必须独立审阅。

## 原型对照

```bash
node frontend/visual/prepare-prototype.mjs
python3 -m http.server 8080 --bind 127.0.0.1 --directory /private/tmp/maulink-phase10-prototype
```

生成临时冻结副本：保留原型 layout/CSS，固定 clock/demo random、停止演示 intervals、隔离偏好、统一服务器数量/新增编辑默认值/文件清单/传输有无。`source.json` 记录原始文件 SHA256 和改动，不修改 `UI/MauLink_prototype_local.html`。本机原型源未纳入 Git，其他环境需提供同 SHA256 的原型，否则不能宣称相同参考条件。

20 个原型状态有稳定 hash，由 `captureReferences` 生成相同 CSS 视口、DPR、主题/语言的 JPEG。原型只在 boot 读取 hash，因此每次导航必须 reload；四个 Settings 还检查页面 heading，防止错误路由落到 Language。原型自身 `.app` min-width=960px，在 CSS 860×640 出现溢出；如实记录，不把原型截图放大或改样式。参考数据能力与 Rust 不同，不对两种实现要求字节/像素相等，也不把示意数据当 Core 实测。

## 校验与画廊

```bash
node --test frontend/visual/capture.test.mjs
node frontend/visual/verify.mjs
```

校验 canonical manifest、完整数量、JPEG 真实像素尺寸、SHA256 和独立重复 hash，拒绝未审阅 candidate/repeat。成功生成 `docs/refactor/screenshots/phase-10/summary.json` 与 `index.html` 双列对照画廊。画廊可用同目录静态 HTTP 服务打开，也可保留相邻目录直接查看。实际 QA/差异见根目录 `design-qa.md`；Browser visual 与真实 Desktop integration 分开记录。

## 后续品牌更新

2026-10-01 顶部 logo 已替换。上述 Phase 10 图片保留历史 M 标记，当前主题实测及严格截图回归的环境限制见 `docs/refactor/frontend-logo-update.md`；不将历史资料完整性校验等同于新界面像素回归通过。
