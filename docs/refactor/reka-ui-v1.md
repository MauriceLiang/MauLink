# Reka UI 组件与交互优化验收报告

日期：2026-10-02。执行依据：`MauLink_前端组件与交互优化方案_RekaUI_v1.0.md`。
基线：`main` / `d1da0f22ed536c7b78adebc6c3828a98eaa19c56`。工作分支：`codex/reka-ui-polish`。

**状态：8 个阶段的代码已实现，自动化和 Browser 验收通过，macOS app 已构建并获用户确认；跨平台最终验收仍为 BLOCKED。** 用户确认后同步更新项目 README，并授权提交推送本轮改造。

## 根因与修改

原生 Select 的展示依赖系统；点击菜单混用了 Context Menu 名称和手工定位；Dialog 自维护焦点；单条 notice 不能排队；控件、提示和动画缺少统一约束。本轮以现有 Base 组件封装 Reka UI，保留已确认的布局与语义配色。

- 精确安装 `reka-ui: 2.10.5`；保留 `typescript: 5.9.3`，Vue 和其他已有依赖版本不变。
- 7 个 Select 迁移到 BaseSelect，保留原业务值（包括分组的 null），支持 placeholder、禁用项、键盘选择、Portal 和碰撞检测。
- FileMenu / ServerCard 使用 BaseDropdownMenu；BaseContextMenu 留给真正右键操作；删除底部行手工翻转 CSS。
- BaseDialog / BaseAlertDialog 使用 Reka 模态层、焦点圈和 Esc。保留 busy 保护和外部 API；删除服务器、分组、文件以及 Host Key 变化确认采用 AlertDialog，初始焦点优先取消。
- Toast 支持并发队列、四种语义、不同 duration、暂停计时、关闭和可选操作；错误与风险仍在需要用户决策的位置显示。
- Settings / Terminal / Connection 控件使用 Checkbox 或 Switch；Server Advanced 使用 Collapsible；输入和按钮保持原生行为，补齐焦点、禁用、加载视觉。
- TooltipProvider 统一延迟；纯图标按钮使用 Tooltip 和 SVG。增加轻量 Popover、Inline Alert 和独立 DEV Interaction Harness。
- 新增 motion / radius / shadow / z-index token 与 overlays.css；Portal 继承 Light / Dark，并在 Dialog 打开时获得正确层级；Reduced Motion 只作用于本轮组件，不全局覆盖 xterm。

Browser 回归发现并修正两处组合问题：Tooltip 嵌套在 DropdownMenuTrigger 外侧会改变 Reka Popper 锚点，菜单跑到屏幕外；现改为 DropdownMenuTrigger as-child 包裹正确转发 DOM ref 的 BaseIconButton。菜单动作在关闭恢复 trigger 焦点后才发出，保证随后打开 Dialog 的返回焦点。非交互 Tooltip 先截获 Esc 时，将关闭意图交给所属 Dialog；交互 Select 的 Esc 仍只关闭 Select。

## 修改文件

- `frontend/package.json` / `package-lock.json`：Reka 依赖。
- `frontend/src/components/base/`：Select、DropdownMenu、ContextMenu、Dialog、AlertDialog、Tooltip、Popover、Checkbox、Switch、Collapsible、Alert、Toast、ToastViewport、UiProvider、Icon 和原 Button / IconButton。
- `frontend/src/composables/useToast.ts`、`src/app/AppShell.vue`、`src/main.ts`：队列和顶层 Provider。
- `frontend/src/dialogs/` 与 server / files / terminal / connection / monitor 相关组件：使用基础组件，保留业务接口。
- `frontend/src/styles/`：tokens、overlays 和必要的原样式调整。
- `frontend/src/harness/`、`frontend/tests/`、`vitest.config.ts`：组件预览、真实 Portal 测试及 jsdom 缺失的浏览器 API 补齐；不 mock Reka 交互。
- `frontend/visual/capture.mjs` / `verify.mjs`：适配 Select 可见标记，新增 implementation-only 验收模式，保留原默认基线验收模式。
- `docs/refactor/screenshots/reka-ui-review/`：本轮截图、manifest、汇总和 gallery；不替换已接受的配色基线。

Rust Core、IPC Contract、Cargo 文件、Tauri 配置及项目 README 没有修改。Command Palette、终端标签键盘行为、连接、SFTP、监控业务沿用原实现。

## 自动化检查

每阶段按 type-check → test → build 执行，修复失败后重验：

| 阶段 | 内容 | type-check | test | build |
| --- | --- | --- | --- | --- |
| 1 | 基础设施 | PASS | 128/128 | PASS |
| 2 | Select | PASS | 130/130 | PASS |
| 3 | Dropdown Menu | PASS | 130/130 | PASS |
| 4 | Dialog / AlertDialog | PASS | 130/130 | PASS |
| 5 | Toast | PASS | 132/132 | PASS |
| 6 | Checkbox / Switch / Collapsible | PASS | 136/136 | PASS |
| 7 | Tooltip | PASS | 136/136 | PASS |
| 8 | Motion / Popover / Harness | PASS | 136/136 | PASS |

最终交付检查：`npm run type-check`、`npm run test`（15 文件，136/136）、`npm run build`、`git diff --check` 均 PASS。`node --test frontend/visual/capture.test.mjs` PASS。

## Browser 实测

在 Codex 内置浏览器中使用真实 click / Tab / Shift+Tab / Arrow / Enter / Space / Esc 操作 DEV Mock Harness：

- Button 焦点可见；disabled / loading 不参加普通 Tab 导航、不触发 action。
- Select 跳过禁用项、选择更新、Esc 关闭、焦点恢复；Dialog 内 Select 通过 Portal 展示，Dark 背景与 z-index 910 正确，选项可点击，Esc 不误关父 Dialog。
- Dialog 初始焦点合理；末尾 input 的 Tab 回到首个焦点元素，Shift+Tab 反向闭环；禁用项跳过；普通 Esc / 遮罩关闭，busy Esc / 遮罩阻止关闭；关闭恢复 trigger。
- AlertDialog 初始取消；分组删除取消后回到管理界面，再关闭仍恢复到“管理分组”。
- Dropdown 支持禁用项跳过、方向键循环、选择、Esc / 外部点击、焦点返回；服务器 Edit / Delete 鼠标操作正常。
- 窄窗口 860×640 的末尾文件菜单自动 `data-side="top"`，矩形位于视口内，实际命中 menuitem，无横向溢出；原手工翻转规则已移除。
- 真正右键菜单、Popover 打开和 Esc 返回、Collapsible 展开、Checkbox / Switch Space 切换正常。
- Tooltip 焦点显示和离开隐藏，DOM 不产生嵌套 button；三条 Toast 同时存在并分别自动移除。暂停、手动关闭与操作回调另由自动化验证。
- Light、Dark、System-Light 实测通过。当前操作系统是 Light，浏览器接口不支持本轮所需的系统媒体模拟，**System-Dark 与操作系统 Reduced Motion 尚未实际验收**；相应 CSS 分支已实现并检查。

## Visual Capture / Verify

命令：

```sh
node frontend/visual/verify.mjs docs/refactor/screenshots/reka-ui-review --implementation-only
```

结果：23 页面 × 8 条件（Light / Dark、zh-CN / en、1440×920 / 860×640），共 **184 张**，DPR 1；两次独立加载截图 byte-identical，overflow 数组为空。检查了代表性截图，未发现布局裁切。详见 [截图汇总](screenshots/reka-ui-review/summary.json) 和 [gallery](screenshots/reka-ui-review/index.html)。

窄窗口 file-delete 初次重复截图存在背景表格滚动条淡出造成的差异；仅在 DEV VisualHarness 固定滚动条，重新捕获窄窗口四组后通过，宽窗口受影响用例也复验通过。初次失败材料留在 diagnostics，生产样式不受此归一化影响。

## Tauri 构建与实测

macOS arm64：`cargo-tauri build --bundles app --ci -- --locked` PASS。

- 应用：`target/release/bundle/macos/MauLink.app`（22.58 MiB）。
- 最初打包产物的严格签名检查报告资源签名过期；对最终 app 完整 ad-hoc 重签后，`codesign --verify --deep --strict --verbose=2` PASS（valid on disk / satisfies its Designated Requirement）。不是公证分发产物。
- 最终可执行文件 SHA-256（含顶部提示修正）：`cbf8ae7107c9b54af86811242ec4cd346217b1f6988ea4dd18bfc433b939b894`。
- 本轮没有代替用户操作原生应用；用户在收到修正版 app 后回复“确认”，同意发布本轮改造。不把这条整体确认扩展为未单独反馈的系统主题、Reduced Motion 或完整 WebView 故障矩阵专项验收。

Windows：尝试 `cargo-tauri build --target x86_64-pc-windows-msvc --bundles nsis --ci -- --locked` **FAIL**。本机缺少 MSVC C SDK 头文件，ring 编译报 `fatal error: 'assert.h' file not found`；没有生成 Windows 安装包。需要具备 MSVC / Windows SDK 的 Windows 构建环境，未擅自增加 CI 或升级依赖绕过。

## 用户复核与剩余风险

用户已确认本轮 macOS app 与顶部提示，并允许更新 README 和提交推送。此前提供的复核范围为：设置中的主题 / 图标 / 语言选择和切换控件；服务器高级设置、密码显示、菜单和删除确认；Dialog 的 Tab / Shift+Tab / Esc 及焦点返回；文件列表底部菜单。系统 Dark、系统 Reduced Motion、真实连接与双平台专项验收仍需独立证据。

Vite 保留大于 500 kB 的 chunk 警告（入口约 710.21 kB，gzip 203.35 kB）；未扩大本轮范围拆分业务模块。Browser 与 WebView 可能存在平台差异，不能以 Browser 结果冒充桌面实测。

**最终：代码、自动化、Browser、视觉矩阵和 macOS 构建 PASS，macOS 应用展示经用户确认；Windows 构建 BLOCKED；系统 Dark / Reduced Motion 与完整桌面专项验收仍未完成。用户已授权更新 README 和提交推送。**

## 用户复核修正：顶部居中操作提示

用户明确要求替换原右下方宽提示条，因此覆盖方案中原 Toast 的位置建议：提示位于顶部工具栏下方 12px、水平居中，按文字宽度收缩，多条垂直排列。文字和图标以绿色成功、蓝色信息、橙色警告、红色错误区分，并使用轻量同色背景与边框。语义色与主文字色混合，确保浅深主题中颜色清晰；空白 viewport 不拦截页面点击。保存设置及快捷切换主题 / 语言后显示绿色成功提示；危险操作确认仍使用 AlertDialog。

只调整 `styles/overlays.css` 和 `app/AppShell.vue` 的提示样式与成功类型。重新执行 type-check、136 项测试、build、diff 检查均 PASS。Browser 实测四色文字、居中、并发排列、关闭及自动消失通过（普通提示高 44px，短文本宽约 138–164px）；Toast 的 8 组视觉截图重新捕获，两次独立加载 byte-identical，完整 184 张 verify 仍 PASS。macOS app 已重新构建并完整 ad-hoc 签名，严格签名校验 PASS，用户随后确认。Windows 与系统媒体验收状态沿用上文。
