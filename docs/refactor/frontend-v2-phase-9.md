# Phase 9 验收报告：Settings、i18n 与 Command Palette

检查日期：2026-10-01（Asia/Shanghai）

状态：**PASS**。type-check、115/115 tests、build 与 Browser 已验证操作通过；Browser 工具无法完成的终端焦点 Ctrl/Cmd+K 已由用户在本阶段独立 Tauri release 应用实测补齐。用户对列出的 1–7 项反馈“没有问题，继续”，包括真实 SSH、命令面板、主题/语言、终端设置、菜单/窗口与应用重启持久化。可以提交推送并进入 Phase 10。

前置：Phase 8 `7935301829a602283a63dc4937eed346f79fe8d7` 已公开推送 main；本阶段不改变 Phase 6 的未验收项。用户先前取消 Desktop 手动门禁，本次主动要求独立应用并完成实测。

## 1. 根因

- Shell 设置入口尚未启用，主题和语言尚未统一读取 Rust SettingsService；Cmd/Ctrl+K 只聚焦服务器搜索。
- 既有模块有部分双语 catalog，但 Shell、连接挑战、终端、文件与监控仍有界面固定文案。
- 文件菜单拥有自己的焦点循环，服务器菜单使用 details，尚未复用统一 Context Menu。
- 实施时发现设置预加载暴露了首次终端创建的时序问题：settings record 已存在时，创建尝试可能在 connection busy 期间被跳过。改为等待 visible、ready 且非 busy，避免依赖设置读取的偶然延迟；原监控不重建 xterm 的回归测试保留并通过。
- Browser 与测试均发现 Palette 首次打开时焦点落到 Close 按钮。BaseDialog 增加可选 initialFocus，Palette 指定 combobox，原 Tab trap、busy Esc 与返回焦点规则保持。

## 2. 修改内容

- 根组件共用既有 TerminalPreferences 的 SettingsRecord，启动调用 settings_get；主题、语言、断开确认和终端设置均通过 settings_update、expectedRevision 提交，保留未修改字段及 opaque directory token。
- 通用、外观、终端、语言四分类复用 BaseDialog；保存后应用，未保存草稿不改变当前主题/语言。冲突保留草稿并提供显式重新加载，不自动覆盖 Core。
- system/light/dark 复用既有 CSS media query；locale 与 document.lang 来自已确认的 Rust record，活动 xterm 立即应用字体、光标和 scrollback。
- localStorage 仅保留原选中即复制偏好。通用、主题、语言保存不写 browser storage；没有存放凭据、服务器配置、私钥内容/路径或 SettingsRecord。
- Shell、Settings、Palette、连接 UI、Terminal、Files、Monitor 按模块 catalog 组织，以稳定 key、参数插值渲染 Vue。沿用既有 Servers、ConnectionState、Errors catalog，未引入运行时 DOM 翻译或大规模中文字符串替换表。
- 服务器名、远端文件名与路径、Core 数据保留原样；单终端/传输数量有独立 English 文案。终端标签采用稳定的 Terminal N 名称，避免语言切换重建终端。
- Palette 保留服务器搜索、工作区/Monitor/Files 导航、添加服务器、主题/语言切换、清除已完成传输，增加 Settings 入口。NFKC 多词搜索、↑/↓ 跳过 disabled、Enter、无匹配、Esc、焦点恢复由统一组件实现。
- Ctrl/Cmd+K 在普通 UI 打开 Palette；来源在 .xterm 中或已有 Dialog 时放行，不截获远端快捷键。终端的 Tab、Arrow 等输入规则未改变。
- 文件与服务器菜单复用 BaseContextMenu：DOM 顺序、禁用跳过、Home/End/Arrow、Tab、Esc、外部点击和 trigger focus restore。通知继续使用 BaseToast；关闭标签随语言变化，文件列表的内联操作反馈保留。
- DEV-only Settings Harness 使用内存 Settings/SSH fixture，带读失败与 revision 冲突场景，正式 bundle 排除。错误 fixture 精确满足 AppError 的 action/details 契约，不绕过 ErrorMapper。

## 3. 修改文件

```text
frontend-v2/src/app/{AppShell,TopBar,Sidebar,ServerNavigation,WelcomeView,LocalBackendStatus}.vue
frontend-v2/src/app/palette.ts
frontend-v2/src/components/base/{BaseDialog,BaseToast,BaseContextMenu,CommandPalette}.vue
frontend-v2/src/components/connection/{ConnectionPanel,ConnectionError}.vue
frontend-v2/src/components/server/{ServerCard,ServerList}.vue
frontend-v2/src/components/files/{FileMenu,FilesPanel,TransferPanel}.vue
frontend-v2/src/components/monitor/MonitorView.vue
frontend-v2/src/components/terminal/{TerminalTabs,TerminalWorkspace}.vue
frontend-v2/src/dialogs/{SettingsDialog,TerminalSettingsDialog,ConnectionDialogs,ServerDialog,GroupDialog,ConfirmDialog}.vue
frontend-v2/src/i18n/{locale,shell,settings,palette,connection,terminal,files,monitor,servers,connections,errors}.ts
frontend-v2/src/errors/presenter.ts
frontend-v2/src/monitor/view.ts
frontend-v2/src/terminal/{preferences,controller}.ts
frontend-v2/src/styles/settings.css
frontend-v2/src/main.ts
frontend-v2/src/harness/{SettingsHarness.vue,settings-fixtures.ts}
frontend-v2/tests/{settings,shell}.test.ts
frontend-v2/README.md
docs/refactor/frontend-v2-phase-9.md
docs/refactor/screenshots/phase-9/*.jpg
```

Core、Contracts、旧 frontend、正式/隔离 Tauri 配置、依赖和 lockfile 均未修改。TypeScript 仍精确 5.9.3。

## 4. 自动化检查

| 检查 | 结果 |
| --- | --- |
| npm run type-check | PASS |
| npm run test | PASS，10 files、115/115 tests；本阶段新增 11 |
| npm run build | PASS，126 modules；CSS 29.39 kB、JS 507.59 kB，gzip 143.51 kB |
| cargo test -p maulink-core settings::tests --locked | PASS，2 tests；其余 filtered 不计通过 |
| 正式 bundle 不含 Settings DEV fixture / 控件 | PASS |
| git diff --check | PASS |

新增测试覆盖 revision/未修改字段、storage 边界、冲突保持旧值与 reload、并发保存拒绝、草稿隔离、读取失败后的重试入口、双语参数一致、NFKC 搜索、禁用导航、Palette 初始 focus / Enter / Esc / restore、无匹配、终端快捷键放行与普通 UI 行为、统一菜单导航与事件。

旧 Shell Cmd/Ctrl+K 聚焦搜索的断言按本阶段需求改为 Palette combobox focus；没有删除替代验收。初始化命令白名单明确增加 settings_get 并仍检查没有 connection_start。jsdom 对 Canvas、Node localStorage 的既有提示不计失败；新测试使用明确内存 storage。所有检查在最终源代码上重新执行。

## 5. Browser 实测

Codex 内置 Browser，真实 UI click/fill/select/press；`?harness=settings` 为显式内存 Mock，不是实际 SSH 或真实数据库。

**已通过：**

- Dark 保存后 dataset.theme=dark、color-scheme=dark、canvas=#0d0f12；重开外观选项仍为 Dark。Light 保存后 color-scheme=light。
- System 保存后 dataset.theme=system，当前系统 prefers-color-scheme: dark=false，计算样式为 light；未改变用户操作系统主题以测试另一方向。
- English / 中文保存后更新 Shell、服务器、设置、终端、文件、Monitor、Palette 与 Toast；远端中文文件名保留原文。English 状态栏为 1 terminal。
- 终端设置保存 18px、Bar、12000 行，关闭再打开值保持；连接打开真实 xterm 渲染器。活动终端字号更新到 20px，.xterm-rows 计算字体为 20px，尺寸从 71×18 变为 64×15，xterm 数量保持 1，保存后恢复 Terminal input 焦点。
- 普通 UI 的 Meta+K 实际打开 Palette，初始 focus=combobox。搜索“打开”时工作区/监控/文件禁用且跳过，Enter 打开 Settings；搜索 files 后 Enter 切换 Files。
- ↑ 从第一项回到最后可用项 Switch language，跳过禁用 Clear completed transfers；↓ 回到 Web-01。无匹配 Enter 不执行；Esc 关闭，焦点回到 Command palette trigger。
- 文件菜单跳过禁用 Download，初始 Copy path；End 到 Delete，Esc 回 empty Actions。服务器菜单 Edit / Delete Arrow 导航与 Esc trigger focus restore 通过，未执行删除。
- 读取失败显示安全设置文案、保留显式 reload 入口。revision 冲突不改变已确认主题，草稿仍为 Dark；恢复后保存成功。
- 860×640 中文 Dark / English Light 和 Dark，无页面横向溢出；1280px English 文件页无横向溢出，用户内容未翻译。截图归档在 phase-9/。

**Browser 工具限制：终端焦点下 Ctrl/Cmd+K。此项由后续用户 Desktop 实测补齐。**

CUA 对可见且已聚焦的 Terminal input 执行 Control+k 和 Meta+k 都返回 selector deadline exceeded；页面仍显示终端输入焦点，但这不足以证明键已送达。通过 body 输入的替代操作也失败（Element is not focused）。普通搜索框 Meta+k 操作成功，终端目标下的自动化分支检查成功，但不将其冒充完整 Browser 实测。没有用 evaluate 派发假事件或修改页面来获得 PASS。

## 6. Tauri 实测

2026-10-01 用户要求提供应用进行手动测试，已重新构建当前 Phase 9 的独立 release 包。用户随后对以下 1–7 项反馈“没有问题，继续”，Native 实测按用户反馈 PASS；不是 Codex 自行操作原生 GUI。

- 命令：`/private/tmp/maulink-tauri-tools/bin/cargo-tauri build --config src-tauri/tauri.frontend-v2.conf.json --bundles app --ci -- --locked`，PASS。
- 产物：`target/release/bundle/macos/MauLink Frontend v2.app`，19.88 MiB，arm64；标识 `io.maulink.frontend-v2.dev`。内嵌生产 frontend-v2，无需 Vite。
- 初始产物保留 linker ad-hoc 签名，完整 bundle 签名检查失败。对该本地测试产物执行 `codesign --force --sign -` 后，`codesign --verify --deep --strict --verbose=2` PASS；未进行开发者证书签名或公证。
- 旧 frontend、正式/隔离 Tauri 配置及依赖 lockfile 的 diff 均为空。
- 用户实测 PASS：①真实 SSH 连接与终端输入；②终端焦点下 Cmd+K / Ctrl+K 不打开 Palette、输入继续可用；③普通 UI Cmd+K、搜索、方向键、禁用跳过、Enter、Esc 与焦点恢复；④主题/双语保存生效及取消草稿；⑤活动终端字号 18、竖线光标、12000 行缓冲；⑥服务器/文件菜单键盘、窗口拖动与缩小布局；⑦Cmd+Q 退出并重启后的主题、语言与终端设置持久化。

Rust SettingsService 已由真实临时 SQLite 的更新、冲突、通知和重开持久化测试覆盖；Browser fixture 的同实例重开 Dialog 不能证明真实应用重启持久化；后者由用户第 7 项实测确认。

## 7. 剩余风险

- Browser 工具仍无法向 Terminal input 完成 Ctrl/Cmd+K；保留工具失败记录，对应行为已由自动化与用户真实 Desktop 操作验证。
- JS 主包 507.59 kB 超过 Vite 的 500 kB 提示阈值，构建成功。本阶段未通过调高阈值隐藏警告或引入无关拆包改造。
- macOS Native GUI 已由用户确认；Windows 键盘与操作系统实际主题切换未现场验证。
- DEV 内存 fixture 在整个 Harness 重载后重置，不表示正式 Rust 数据会丢失。
- Phase 6 Ctrl+C / 突发输出等已知验收缺口保持。

## 8. 退出结果

**PASS**。此前 Browser 工具限制由用户 Tauri 实测补齐；按用户持续授权提交推送 main，并进入 Phase 10。Phase 6 的既有验收缺口保持单独记录，不随本阶段通过而消失。

推荐 Commit：`feat(frontend-v2): migrate settings i18n and command palette`。
