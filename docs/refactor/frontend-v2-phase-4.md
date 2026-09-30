# Phase 4 验收报告：Server Home 与 Server CRUD

检查日期：2026-09-30（Asia/Shanghai）

状态：**PASS：实现、自动化、Rust 回归、Browser 与真实 Tauri Desktop 验收全部通过。用户于本轮反馈“一切正常”，确认下面全部 Desktop 检查。允许提交推送并进入 Phase 5。**

前一阶段 Phase 3 全部 PASS，已提交并公开推送 `61d3347e28dd1f860e9f97af056a021215ca83ed` 到 main；当时 HEAD / origin/main 一致、工作区干净。用户已明确授权后续通过阶段继续公开推送代码、报告和 UI 截图到 MauriceLiang/MauLink。

## 1. 完成项与修正原因

- 读取手册 Phase 4、旧 Server 页面和表单/凭据/删除逻辑、Server/Group/Credential DTO、Tauri commands、Core profiles/credentials/local_files、design-qa.md Page 3/4/5 和旧最终 CSS。
- 建立 Vue refs 的 serverStore，仅管理 Server/Group/UI 状态，沿用 200 条分页和本地搜索；分页全部完成后再替换列表，失败不写入不完整资料。
- 接入真实 typed serverApi，完成 Server Home/Card、资料查看、新增/编辑/删除确认，以及分组创建/重命名/删除确认。无散落 invoke，无 Terminal 输出或新增状态库。
- 编辑先 server_get 获取最新 revision，更新/删除携带 expectedRevision；冲突不自动重试覆盖，保留修改并提供显式重载。ServerInUse、未知错误安全展示，Rust debug details 不展示。
- 明确区分加载、验证、提交、错误与成功通知。提交时禁用表单，防重复提交，busy 时沿用 BaseDialog 的 Esc 规则。
- 编辑明确保留/替换/移除系统凭据；身份变化时禁止默认 keep 旧凭据。密码和口令仅在当前表单暂存，保存或关闭后清空引用；不记录到 Store、localStorage 或日志。
- 私钥仅使用 local_file_select 的 token/displayName/purpose/expiresAtMs；不读取原始内容/路径。支持保留已有私钥、选择取消、token 到期提示、后端 token 消费。
- credentialCleanupPending 不伪装成完全清理成功，通知明确“操作已完成，但旧凭据清理尚未完成”。本阶段不扩展完整凭据管理 UI。
- 复用 Base 组件、旧图标和 token；服务器组件/表单建立中文/英文 catalog 与 language 参数，完整应用语言切换仍在后续阶段。
- 添加开发专用 Mock CRUD Harness；Native Harness 使用真实 IPC，并以受限回环占用探针完成真实 ServerInUse 验收所需状态。该探针调用已有 test-mode commands，不迁移产品的 Connection UI、不改 SSH Core、不进入正式构建。

运行态修正：BaseInput 将 class 传给 input，最初导致表单全行字段排错位置，改为外层布局容器；分组选择放在高级区，使默认弹窗与旧版尺寸接近；删除后原 trigger 消失时，服务器焦点回 Home，分组焦点回名称输入；Harness 控件移至通知上方，避免保存 Toast 覆盖。均已重新检查。

## 2. 修改文件

```text
frontend-v2/src/stores/servers.ts
frontend-v2/src/components/server/ServerCard.vue
frontend-v2/src/components/server/ServerList.vue
frontend-v2/src/dialogs/ServerDialog.vue
frontend-v2/src/dialogs/ConfirmDialog.vue
frontend-v2/src/dialogs/GroupDialog.vue
frontend-v2/src/dialogs/server-form.ts
frontend-v2/src/i18n/servers.ts
frontend-v2/src/i18n/errors.ts
frontend-v2/src/app/AppShell.vue
frontend-v2/src/app/TopBar.vue
frontend-v2/src/app/Sidebar.vue
frontend-v2/src/app/ServerNavigation.vue
frontend-v2/src/app/WelcomeView.vue
frontend-v2/src/components/base/BaseDialog.vue
frontend-v2/src/harness/ServerHarness.vue
frontend-v2/src/harness/server-fixtures.ts
frontend-v2/src/harness/occupancy-probe.ts
frontend-v2/src/harness/ShellHarness.vue
frontend-v2/src/styles/servers.css
frontend-v2/src/main.ts
frontend-v2/tests/server-management.test.ts
frontend-v2/tests/shell.test.ts
frontend-v2/README.md
docs/refactor/frontend-v2-phase-4.md
docs/refactor/screenshots/phase-4/*.jpg
```

BaseDialog 只新增可选 panelClass，默认行为不变，Phase 2 键盘测试保留。ShellHarness 明确只读，避免旧开发入口误发写入命令。原 Shell 测试仅更新本阶段已启用 CTA/Server Home 的预期，不删除替代验收。

## 3. 新增依赖

无。使用既有 Vue refs、Base Components、typed facade 和 contracts/v1。TypeScript 仍精确固定 5.9.3；package.json / lockfile 未变。未增加 Pinia、Router、文件系统权限或 Tauri capabilities。

## 4. 自动化检查

| 命令/检查 | 结果 |
| --- | --- |
| npm run type-check | PASS |
| npm run test | PASS，5 files、38/38 tests |
| npm run build | PASS，64 modules；HTML 0.42 kB、CSS 14.41 kB、JS 116.55 kB |
| cargo test --workspace --locked | PASS，68 passed、11 ignored；ignored 不计为通过 |
| git diff --check | PASS |
| 旧 frontend / Core / Tauri / Contract / 依赖完整性 | PASS，git diff --exit-code -- crates src-tauri frontend contracts frontend-v2/package.json frontend-v2/package-lock.json 无差异 |
| 正式 bundle 排除 Mock/Native Harness | PASS，不含 Mock Server CRUD、fixture_ed25519、开始回环占用验收或 Phase4-占用验收 |
| 新实现 invoke / 浏览器凭据持久化检查 | PASS，Store/Dialogs/Server Components 没有直接 invoke、localStorage 或 sessionStorage |

新增 15 个实际用例，覆盖提交结果与 revision、搜索不请求 IPC、后续分页失败保留完整列表、端口/高级字段/凭据身份验证、token 过期与已有私钥保留、真实 payload、transient secret 清空、加载最新 revision、cleanupPending 通知、三类错误、重复提交/busy Esc、删除确认与消失 trigger 的焦点、分组 CRUD/保留成员、回环探针拒绝远程/凭据/代理配置及使用最新 revision/test mode。

初次新增测试 helper 的默认参数被推断为带 commands 的具体 Mock 类型，type-check FAIL；显式使用 IpcTransport 后修正，重新按顺序通过 type-check/test/build。Vitest 的未来 native config loader 导入扩展名提示仍为非阻塞提示，未扩大配置修改范围。

Rust 未修改；本次只报告真实运行的新 workspace test 结果，不把 prior results 重复作为新证据。11 ignored 包括系统凭据和隔离 OpenSSH 集成测试，原生钥匙串及当前 WebView 行为仍必须由 Desktop 验收补足。

## 5. Browser / Desktop 实测

### Browser：PASS

Codex 内置浏览器，通过真实 click/fill/select/press 操作：

- 空列表/有服务器 Light/Dark；服务卡片与侧栏一致，查看仅展示资料和“尚未连接”。
- 新增 Browser-P4 密码 fixture，空 Host 验证阻止保存；保存成功后列表/侧栏同步，搜索 browser.example 仅保留该记录。
- 编辑 Host 时 keep 已保存凭据被阻止，明确 clear 后成功；随后改名成功且恢复原编辑 trigger 焦点。
- 新增 Browser-Key，SSH 密钥切换、选择演示 token、保存并显示卡片通过；高级字段与分组选择通过。
- 分组创建/重命名通过；删除 Production 先显示确认，确认后两台成员移至未分组、服务器数量保留。
- 服务器删除取消后记录保留，确认后消失；成功后焦点回 Home。
- Mock ServerInUse 和 RevisionConflict 安全文案正确，冲突显式重载清除错误；未知错误只显示安全通用文案。
- 初始焦点合理；Tab 从最后 Save 回到 firstFocusable，Shift+Tab 从 firstFocusable 回到 Save；Esc 关闭后恢复 trigger。
- 延迟 10 秒提交时 aria-busy=true、fieldset disabled=true；真实对话框 Escape 后仍打开，回应后正常关闭。测试驱动一次默认 hidden 等待早于 10 秒超时，随后确认提交完成；不将该驱动等待计为产品失败或 busy 通过证据。
- 1440×920 与 860×640 通过；最小尺寸 scrollWidth=860，卡片单列，无横向溢出。默认密码弹窗 526×586.75，最小窗口内 top=26.625、bottom=613.375，Save 可见；扩展表单可滚动，Tab 自动滚动焦点到可见区域。
- 各最终 Browser 检查 console warning/error 空；临时标签与 viewport 已清理。

Browser 的私钥和安全存储使用 Mock，不作为真实系统文件选择或钥匙串写入证明。

### Tauri：PASS（用户实际操作确认）

已使用专用 frontend-v2 配置与 `/private/tmp/maulink-phase4-native.json` 临时 devUrl overlay 启动：

```bash
PATH=/private/tmp/maulink-tauri-tools/bin:$PATH cargo tauri dev --config src-tauri/tauri.frontend-v2.conf.json --config /private/tmp/maulink-phase4-native.json -- --locked
```

URL 为 `?harness=servers&transport=native&theme=light`。右侧“Native Server CRUD 验收”使用真实后端，不是 Mock。窗口参数、正式入口和 capability 不改。Vite ready，Rust dev 编译 PASS（8.35s），target/debug/maulink 进程已运行；不能据启动推定 GUI PASS。

临时 TCP fixture 只监听 127.0.0.1:42424，接受连接后等待关闭，不回应 SSH banner、不提供登录；用于维持真实连接进行中状态。一次性 ED25519 私钥 fixture 在 `/private/tmp/maulink-phase4-key`，不在仓库内，不用于任何真实账户。

用户操作清单：

1. 新增 `Phase4-Password`，主机 `password.example.test`、用户 `phase4`、密码使用一次性测试值；确认保存成功及卡片出现，重新打开编辑仍显示资料。系统授权提示由用户处理。
2. 新增 `Phase4-Key`，主机 `key.example.test`、用户 `phase4`、认证 SSH 密钥、口令留空，通过系统选择器选择 `/private/tmp/maulink-phase4-key`。保存后编辑名称时不重新选择，确认“保留当前私钥”和保存正常。
3. 编辑密码服务器的名称；再编辑 Host/Port，确认 keep 旧凭据时被阻止，明确选择 clear/replace 后可保存。检查保存通知、安全错误文案与 Esc/Tab/焦点恢复。
4. 管理分组：新建、重命名；通过服务器“高级 → 分组”分配记录；搜索应同步两处和卡片。删除分组须确认，成员应保留并移至未分组。删除服务器先取消，确认记录保留，再确认删除。
5. 通过 Native 验收控件切换 Light/Dark，缩到最小尺寸，确认布局、原生交通灯、拖动与表单操作正常。
6. 真实占用验收：新增唯一的 `Phase4-占用验收`，Host=`127.0.0.1`、Port=`42424`、Username=`phase4`、password 且密码留空；高级区 timeout=`120000`，不设置代理/跳板机。展开 Native 控件，点击“开始回环占用验收”，立即打开该配置编辑，改 Port 为 42425 并保存，应显示“服务器正在使用中”且保留原资料。关闭编辑，点击“取消回环占用”，再次改 Port 应可保存。最后删除该临时配置。

用户反馈“一切正常”，上述六项全部通过，包括真实系统凭据、文件选择器、占用时阻止修改、取消后恢复编辑和 WebView 键盘行为。真实占用通过依据是用户操作反馈，不以 Mock 或 TCP 接收日志代替。验收后已通过 Ctrl+C 停止 Tauri 和回环 TCP 进程；TCP 的 KeyboardInterrupt 是主动停止结果。

## 6. 视觉验收与截图

所有图片像素尺寸已由 sips 核对；1440×920 为默认尺寸，带 860 的图为 860×640。

| 状态 | 截图 |
| --- | --- |
| Server Home Light / Dark | [Light](screenshots/phase-4/server-home-light.jpg) / [Dark](screenshots/phase-4/server-home-dark.jpg) |
| Empty Light / Dark | [Light](screenshots/phase-4/empty-light.jpg) / [Dark](screenshots/phase-4/empty-dark.jpg) |
| Add Password Light / Dark | [Light](screenshots/phase-4/add-password-light.jpg) / [Dark](screenshots/phase-4/add-password-dark.jpg) |
| Add Key / Advanced | [Key](screenshots/phase-4/add-key-light.jpg) / [Advanced](screenshots/phase-4/advanced-light.jpg) |
| Edit / ServerInUse Mock | [Edit](screenshots/phase-4/edit-server-dark.jpg) / [Error](screenshots/phase-4/server-in-use-dark.jpg) |
| Group Delete Confirm | [Confirm](screenshots/phase-4/group-delete-light.jpg) |
| 最小尺寸 Home / Dialog | [Home](screenshots/phase-4/server-home-dark-860.jpg) / [Dialog](screenshots/phase-4/add-password-dark-860.jpg) |

使用旧 Server Home 最终 CSS（双列/14px gap/16px card padding）与 [Phase 3 原型截图](screenshots/phase-3/prototype-servers-dark.jpg) 对照 Shell/卡片结构，fixture 数量不同不伪装数据等同。旧新增弹窗 CSS 基线为约526×556；当前宽526、默认密码高586.75，保留 BaseDialog 关闭入口/完整字段语义，原型基本尺寸与层级已获 Desktop 人工认可。

## 7. 与旧前端差异

- 本阶段保存资料，不提供“连接”或“测试连接”产品入口；卡片用“查看”，不伪造 SSH 状态。Connection、安全挑战及 Workspace 留在 Phase 5/6。
- 凭据模式显式展示；所属分组在高级区可选，编辑不默认暴露原凭据。
- 中文/英文 server catalog 和组件参数已预留；完整应用语言切换与 Settings 未迁移。
- 原 Shell 的读文件方式、Core、正式配置、旧 frontend 与 IPC Contract 保留。

## 8. 当前阻塞与剩余风险

当前无阻塞。真实 Tauri Desktop 已由用户完成验收。

非阻塞范围限制：Windows 未运行；完整 English/Settings 未迁移；ignored OpenSSH/native-store 测试未跑；cleanupPending 仅明确通知，完整清理管理 UI 后续迁移。临时 TCP 和 Tauri 进程已停止；一次性 key 留在 /private/tmp，避免删除用户可能保留的测试资料所引用文件，不提交临时文件或任何测试凭据。

## 9. 是否达到退出条件

**PASS**。type-check、38/38 tests、build、Rust 回归、Browser 和用户 Tauri 实测全部通过，可进入 Phase 5。

## 10. 推荐 Commit

```text
feat(frontend-v2): migrate server management
```

用户明确授权全部阶段条件 PASS 后，自动写完整提交正文、提交并公开推送 main。本报告随本阶段代码一并提交；实际 commit 和推送结果以 Git 记录为准。
