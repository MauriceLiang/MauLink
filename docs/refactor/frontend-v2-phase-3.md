# Phase 3 验收报告：Application Shell

检查日期：2026-09-30（Asia/Shanghai）

状态：**PASS：自动化、Browser、默认 Tauri 与 Shell Mock Harness 的 Desktop 验收全部通过；拖动权限修正后用户复测正常。**

## 1. 完成项

- 已读取手册 Phase 3、旧 index.html 的 Shell、CSS 最终覆盖与断点、导航/分页逻辑、design-qa.md Page 0/1/2、本地 UI 描述和互动原型。
- 将 Phase 1 空壳拆为 AppShell、TopBar、Sidebar、ServerNavigation、LocalBackendStatus、WelcomeView；保留原有结构与层级，未复制整个旧 HTML。
- 复用 Phase 2 BaseButton、BaseIconButton、BaseInput、BaseDialog、BaseEmptyState 及 Typed IPC/Error Mapper。
- 默认入口仅通过 app_get_info、group_list、server_list 读取资料；沿用每页 200 条与 cursor 加载，不发起 SSH，不修改资料。
- 导航分组、未分组、选中状态、Home 返回、两处搜索同步、Cmd/Ctrl K 聚焦搜索与 About 焦点恢复已建立。
- Loading/错误状态不显示虚假的空列表或 ready；安全错误展示支持真实重试。App Version 来自 app_get_info，不硬编码为产品版本。
- 添加独立开发 Shell Mock Harness，提供空列表/有服务器和 Light/Dark 控件；不访问真实服务器，正式构建不启用。
- 使用旧图标系统中的必要 SVG 子集并保留 Lucide 许可证，没有新增图标库或 runtime 依赖。
- 修正运行态发现的 CSS 加载顺序问题：Shell 样式集中在 main.ts 中于 Base Components 之后加载，避免导航按钮被基础面板样式覆盖。已重新检查透明背景与选中状态。
- 修正原生拖动权限缺口：仅在 frontend-v2 专用配置中增加 main 窗口的 core:window:allow-start-dragging，保留 main 业务 capability。正式配置、共享 capability、窗口参数和 Rust Core 不变。

## 2. 修改文件

```text
frontend-v2/src/App.vue
frontend-v2/src/main.ts
frontend-v2/src/app/AppShell.vue
frontend-v2/src/app/TopBar.vue
frontend-v2/src/app/Sidebar.vue
frontend-v2/src/app/ServerNavigation.vue
frontend-v2/src/app/LocalBackendStatus.vue
frontend-v2/src/app/WelcomeView.vue
frontend-v2/src/app/ShellIcon.vue
frontend-v2/src/harness/ShellHarness.vue
frontend-v2/src/harness/shell-fixtures.ts
frontend-v2/src/styles/shell.css
frontend-v2/src/styles/tokens.css
frontend-v2/src/styles/themes.css
frontend-v2/tests/shell.test.ts
frontend-v2/LICENSE-lucide
frontend-v2/README.md
src-tauri/tauri.frontend-v2.conf.json
docs/refactor/frontend-v2-phase-3.md
docs/refactor/screenshots/phase-3/*.jpg
```

## 3. 新增依赖

无。Vue、TypeScript 精确 5.9.3、Vite、Tauri API 与既有测试依赖不变；package.json / lockfile 没有改动。没有加入 Router、Pinia 或 UI 框架。

## 4. 自动化检查

### Frontend

拖动权限修正后重新按顺序执行：

| 命令/检查 | 结果 |
| --- | --- |
| npm run type-check | PASS，退出码 0 |
| npm run test | PASS，4 文件、23/23 项通过 |
| npm run build | PASS，48 modules；HTML 0.42 kB、CSS 10.74 kB、JS 84.27 kB |
| git diff --check / 新增源码尾随空格检查 | PASS |
| 正式构建排除开发 Mock fixture | PASS，dist JS 不包含 Mock IPC 或 dev.example.com |
| 旧 frontend / Rust / Contract / 正式 Tauri 配置完整性 | PASS，git diff --exit-code -- crates frontend contracts src-tauri/tauri.conf.json src-tauri/capabilities/main.json 无差异；专用 frontend-v2 配置仅增加拖动 capability |

新增 5 项测试保护 loading/ready、完整导航分页与只读 command 集合、两处搜索和未知分组、安全错误与真实重试、快捷键/About/焦点恢复与事件监听清理。保留 Phase 2 的全部键盘回归测试。

新增第 6 项 Shell 回归检查：TopBar 原生拖动区域、品牌按钮不作为拖动区域、frontend-v2 配置保留 main 并只向 main 窗口授予拖动权限。修正前该测试失败（capabilities 缺失，5 passed / 1 failed），修正后全套 23/23 通过；该测试不代替原生拖动实测。

Vitest 的未来 native config loader extensionless 导入提示仍为非阻塞提示，未扩大本阶段配置修改范围。

### Rust

未修改 Rust Core、commands、DTO、共享权限或任何 Tauri 窗口参数；仅补充 frontend-v2 专用配置的原生拖动权限。不重复记录旧 Rust 测试为本阶段新通过结果。权限修正后 Tauri `cargo run --locked --no-default-features` dev 编译 PASS（6.94s），进程已运行。不将 dev 编译当作 Rust 全量测试或 UI 验收。

## 5. Browser / Desktop 实际验证

### Browser

Codex In-app Browser 使用真实 press/click/select 操作，均 PASS：

- Empty Light/Dark、Server Exists Light/Dark；两组分组和三台服务器 fixture，所有服务器均为未连接。
- 空态 Tab：品牌 → Home → 全局搜索 → About → 侧栏搜索 → 了解 MauLink；disabled 的添加、设置、分组入口被跳过。
- 有服务器 Tab：侧栏搜索 → Web-01 → DB-01 → Dev-01。Enter 选中后显示资料及“尚未连接”，aria-current 正确；Home 返回。
- 全局输入 dev.example 后，仅保留 Dev-01，侧栏输入同步；清空恢复完整导航。
- Cmd K 聚焦全局搜索；About 显示 app info，Esc 关闭且恢复原 trigger 焦点。
- Browser Harness console warning/error：空列表。

### Tauri

用户反馈默认 frontend-v2 **无法拖动窗口，其余正常**：真实 IPC 加载、状态栏与版本、原生交通灯、默认/最小尺寸布局、Tab/Cmd K/About Esc 焦点恢复记录为通过；原生拖动初次验收 FAIL。

**根因**：TopBar 已有 data-tauri-drag-region；Tauri 2.11.6 原生脚本通过 plugin:window|start_dragging 发起拖动，但生效的 main capability 未授予该命令。应用日志反复出现 `window.start_dragging not allowed. Permissions associated with this command: core:window:allow-start-dragging`。withGlobalTauri=false 不会阻止 Tauri 注入原生拖动脚本，无需增加前端手写拖动处理。

**修正**：frontend-v2 专用配置显式保留 main，并增加仅限 main 窗口、仅含 core:window:allow-start-dragging 的内联 capability。未授予整套 core:default，未改正式配置或窗口外观。重启后用户反馈“确认正常”，原生拖动与搜索/按钮复测 PASS。

辅助核对曾尝试从生成的 capabilities.json 查找内联 capability，但该文件只包含文件型 capability，因此该检查失败，不能作为权限未生效的判断。已读取本机 Tauri codegen/get_capabilities 实现，确认编译时会把配置内联 capability 与 main 引用合并并解析 ACL；配置回归与实际编译通过，最终以用户拖动复测为准。

启动命令：`PATH=/private/tmp/maulink-tauri-tools/bin:$PATH cargo tauri dev --config src-tauri/tauri.frontend-v2.conf.json -- --locked`。

1. 默认 frontend-v2 全部检查通过，拖动修正后的用户复测已通过。
2. 使用 /private/tmp/maulink-phase-3-shell.json 临时 devUrl overlay 启动 Shell Mock Harness（?harness=shell&state=empty&theme=light），Rust dev 编译 PASS（6.15s）。用户针对 Light/Dark、空列表/有服务器、导航、搜索、About、键盘焦点和最小尺寸反馈“正常”，Desktop Mock 验收 PASS。

两项 Desktop 验收全部通过，达到 Phase 3 退出条件。验收进程已停止，临时 overlay 不写入仓库。

## 6. 视觉验收与截图对比

实际 CSS 视口及 JPEG 像素尺寸均已核对；默认大图为 1440×920、DPR 1，最小尺寸为 860×640。

| 状态 | 新前端截图 |
| --- | --- |
| Empty / Light | [empty-light-1440.jpg](screenshots/phase-3/empty-light-1440.jpg) |
| Empty / Dark | [empty-dark-1440.jpg](screenshots/phase-3/empty-dark-1440.jpg) |
| Server Exists / Light | [servers-light-1440.jpg](screenshots/phase-3/servers-light-1440.jpg) |
| Server Exists / Dark | [servers-dark-1440.jpg](screenshots/phase-3/servers-dark-1440.jpg) |
| Empty / Light / 860×640 | [empty-light-860.jpg](screenshots/phase-3/empty-light-860.jpg) |
| Server Exists / Dark / 860×640 | [servers-dark-860.jpg](screenshots/phase-3/servers-dark-860.jpg) |

对照证据：

- [旧版 Shell / Light](screenshots/phase-3/legacy-empty-light.jpg)：1440×920；48px Topbar、236px Sidebar、30px Statusbar 与新前端一致。浏览器没有 Tauri，旧版侧栏显示“桌面服务未连接”，首页未完成 Desktop 数据初始化，因此文案与新空态不同；此图只作结构对照，不作为旧版后端验收。
- [原型 Empty / Light](screenshots/phase-3/prototype-empty-light.jpg)、[原型 Empty / Dark](screenshots/phase-3/prototype-empty-dark.jpg)：1440×920；中央 64px 图标、20px 标题、两行说明及留白层级对齐。原型自绘交通灯仅为参考，新前端不绘制重复交通灯。新前端尚未迁移的 CTA 使用 disabled 视觉，原型 CTA 可用，这是阶段范围差异。
- [原型 Server Exists / Dark](screenshots/phase-3/prototype-servers-dark.jpg)：有四台演示服务器/主页卡片，新 Shell 使用三台只读 fixture/资料占位；本阶段对照分组导航与 Shell 容器，主页卡片属于 Phase 4，不伪造其已迁移。
- 旧 Browser 尝试切换深色时，settings_save 因没有 Tauri 被拒绝并恢复 system，已保存为 [旧浏览器服务限制](screenshots/phase-3/legacy-browser-service-unavailable.jpg)。此图不标为深色通过；新 Shell 深色已通过真实 Browser 检查，并与原型深色对照。

布局测量：1440px Sidebar=236px、1080px Sidebar=220px、860px Sidebar=190px；860×640 document scrollWidth=860、Statusbar bottom=640、品牌 left=68、Home 图标隐藏。1440px 品牌 left=78。搜索与操作区不重叠，没有横向溢出。原生交通灯已由用户在默认 Desktop 中确认正常。

Light/Dark 中文与上述视口 PASS；完整 English、Windows 原生窗口和业务页面不在本阶段已验收范围。浏览器临时视口与标签已清理。

## 7. 与旧前端差异

- 只迁 Shell 和只读导航；没有 Server CRUD、真实 SSH Workspace、Terminal、SFTP、Monitor 或完整 Settings。
- 添加服务器、新建分组、设置仍禁用；传输/终端仅保留状态栏外观，不伪造活动任务或已连接状态。
- 有服务器时中央提供资料占位，完整 Server Home 卡片在 Phase 4 迁移。
- Statusbar 增加弱化的真实 App Version；About 复用 Phase 2 Dialog。
- Sidebar 收缩行为按旧实现的 1080/900px 断点，未新增 icon mode；最小宽度与现有 Tauri 860px 一致。
- 未改正式入口、窗口配置、Core 生命周期或数据/凭据存储规则。

## 8. 当前阻塞与剩余风险

无当前阻塞。默认 Desktop、拖动修正后复测、Shell Mock Harness Desktop 均已有用户通过反馈。

遗留非阻塞限制：完整 English/Windows 未测；旧浏览器无法持久化主题；Vitest 未来配置提示仍存在；后续业务范围保持待迁移。

## 9. 是否达到退出条件

**PASS**。type-check、23/23 测试、build、Browser 与全部 Desktop 验收通过，旧入口/Core/Contract/正式窗口配置保持完整。拖动初次验收失败已修正并由用户复测通过，允许继续 Phase 4。

## 10. 推荐 Commit

```text
feat(frontend-v2): migrate application shell
```

按照用户授权，本阶段全部条件 PASS 后以该标题及完整正文提交并推送 `main`，随后继续 Phase 4。提交 SHA 与推送结果由 Git 输出核实，不在提交自身中重复写入 SHA。
