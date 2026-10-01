# Phase 7 验收报告：SFTP 与 Transfer

检查日期：2026-10-01（Asia/Shanghai）

状态：**PASS（按用户更新后的 Browser 门禁）**。type-check、87/87 tests、build、Browser 实测与既有隔离 OpenSSH SFTP 测试通过。Tauri GUI / 系统文件选择器没有实测，不记为实测 PASS；用户已取消后续 Desktop 手动验收门禁。允许自动提交推送 main 并进入 Phase 8。

前置状态：Phase 6 `0e6a51fc035379ec3f879e64cdc3096aadd10db9` 已按用户“直接推送进入”的授权推送 main；其报告中的 Ctrl+C、突发输出及其他验收缺口仍为 BLOCKED，本报告不追溯改记为 PASS。

## 1. 根因与目标

新工作区缺少 SFTP 文件视图和传输任务 UI。本阶段读取既有 Core SFTP / Transfer Manager、生成的 Contracts、Tauri commands、旧 Files / remote-path.mjs 及 design-qa.md Page 9、10、16，迁移现有接口与分页语义，不增加远程编辑后端。

Browser 发现 ArrowDown 打开菜单的按键同时冒泡到菜单导航处理，导致跳过第一项；已阻止该打开事件冒泡，重验第一项焦点、正反向循环与 Esc 恢复。收尾检查补充了迟到轮询的持续恢复、保留活动任务优先裁剪终态历史，以及零字节完成任务的确定进度显示。中途 type-check 指出 DEV 文件 token 缺失字段、Mock Channel cast 和超出当前 lib 的 findLastIndex；分别补齐 Contract 字段、类型与现有目标兼容的循环，未修改 TS / lib 或升级依赖。最终检查全部通过。

## 2. 修改内容

- 在原连接工作区增加“终端 / 文件”入口；切换通过 v-show 保留 xterm、PTY 与传输 Store，回到终端恢复输入焦点。传输 Store 由 AppShell 持有，工作区隐藏不取消任务。
- 文件列表复用 list_start / list_next / list_close。一次替换一页，最多 200 行；下一页沿用 Core cursor，回到第一页重新打开游标，不伪造反向游标、不无限追加 DOM。
- 导航、卸载和迟到响应释放游标；请求世代防止旧目录或符号链接 stat 覆盖新状态。游标过期保留旧页但禁用写操作，刷新成功后恢复。权限错误保持真实失败。
- POSIX 路径只做路径拼接，保留中文、引号和合法反斜杠；不存在 Shell 文件管理。目录链接通过 stat followSymlink 核实后进入。
- 原生可聚焦 Breadcrumb / 路径表单 / 上级目录；键盘菜单支持 ArrowUp/Down、Home/End、Esc、禁用项跳过、关闭后焦点恢复。
- mkdir / rename / delete 使用既有 typed IPC；delete 二次确认携带 confirmed 与 expectedType，符号链接只删除链接。成功后刷新目录，原 trigger 消失时聚焦路径输入；取消保留原文件。
- Upload / Download 通过 local_file_select 取得一次性 token，使用原 Rust Transfer Manager 与 Tauri Channel。下载先 stat 确认普通文件；选择取消或上下文变化不启动任务。Vue 只保存元数据，不读取或保存文件字节。
- 进度、速率、剩余时间、完成 / 失败 / 取消及 cleanupRequired / temporaryPath 明确显示。decimal u64 保留原始字符串，仅显示计算使用近似数值。
- cancel 调真实接口，保持“正在取消…”直至 Channel / poll 确认终态；发布完成竞态允许最终为 completed。迟到 start / poll / cancel 不覆盖更新进度或终态。静默 Channel 使用单任务轮询恢复，旧响应被拒绝也继续轮询。
- 历史元数据优先裁剪终态，保留活动任务；清除已完成只清 UI 元数据，重载历史不复活已清项目。卸载仅释放 UI Channel / timers，不隐式取消 Core 任务。
- 有活动传输时，断开确认必须显式勾选停止传输；默认仍为 stopActiveTransfers=false，用户确认后才传 true。断开后禁止文件写操作，保留任务结果；状态栏显示真实活动任务数量。
- Remote File View/Edit 持续禁用，明确缺少 Core 接口，不伪造预览或编辑功能。
- DEV-only 文件 / AppShell 组合 Harness 支持 450 项目录、失败、慢速取消、取消 picker、游标失效、重置恢复。生产构建排除这些 fixtures。

## 3. 修改文件

```text
frontend-v2/src/components/files/{FileMenu,FilesPanel,TransferPanel}.vue
frontend-v2/src/files/path.ts
frontend-v2/src/stores/{files,transfers,connections}.ts
frontend-v2/src/components/terminal/TerminalWorkspace.vue
frontend-v2/src/app/{AppShell,LocalBackendStatus}.vue
frontend-v2/src/i18n/errors.ts
frontend-v2/src/styles/files.css
frontend-v2/src/main.ts
frontend-v2/src/harness/{FilesHarness.vue,files-fixtures.ts}
frontend-v2/tests/{files,connections}.test.ts
frontend-v2/README.md
docs/refactor/frontend-v2-phase-7.md
docs/refactor/screenshots/phase-7/*.jpg
```

没有依赖变更；TypeScript 精确 5.9.3。Rust Core、IPC Contract、旧 frontend、Cargo 配置与 lockfile、正式及 v2 Tauri 配置、前端 package / lockfile 均无差异。

## 4. 自动化检查

| 检查 | 结果 |
| --- | --- |
| npm run type-check | PASS |
| npm run test | PASS，8 files、87/87 tests，本阶段新增 21 |
| npm run build | PASS，102 modules；HTML 0.42 kB、CSS 25.30 kB、JS 472.15 kB（gzip 130.71 kB） |
| cargo test -p maulink-core --test openssh_sftp openssh_sftp_ -- --ignored --test-threads=1 | PASS，4 passed、2 filtered out，实际隔离 OpenSSH 回环服务 |
| git diff --check | PASS |
| Core / Contract / 旧前端 / Tauri / 依赖文件无差异 | PASS |
| 生产 bundle 无 files-fixture、DEV Harness、模拟任务路径 | PASS |

真实后端测试：208 个目录项分页、Unicode / newline / symlink、游标生命周期；安全 mkdir / rename / delete；上传下载完整性与 no-clobber 发布；取消上传不发布目标并清理临时文件。没有运行 100 MiB / 1 GiB / 10 GiB 的大文件矩阵，没有将过滤项记作 PASS。

新增前端测试覆盖：POSIX 文件名、页替换与 cursor 释放、迟到列表及 symlink stat、失效页禁用写操作、删除类型与确认、取消 picker / 上下文变更、下载目录拒绝、早到 Channel 终态、精确 u64 元数据、异步取消竞态、轮询恢复 / 迟到响应、清除历史、活动任务保留、零字节完成显示、安全错误与清理提示、删除确认与焦点、键盘菜单、显式 stopActiveTransfers。

Vitest 的 jsdom Canvas、Node localStorage、未来 Vite config loader 提示为非阻塞运行时提示；测试退出码为 0，未为消除提示安装无关依赖。

## 5. Browser 实测：PASS

Codex 内置浏览器真实 click / fill / select / press，使用生产组件加 DEV 内存 Mock IPC，**不宣称 Browser 连接了真实 SFTP 或操作了系统文件选择器**。

- 目录 450 项：第一页 200、第二页 200、第三页 50，末页 Next disabled；回第一页正确。
- 目录 / 链接入口、空目录、上级目录、手输路径、Breadcrumb Enter 导航；权限错误安全文案与写操作禁用，重新导航恢复。
- 创建“验收目录”；重命名为含中文、双引号、反斜杠的 basename，复制路径内容保持原字符。
- 删除第一次打开确认后取消：文件存在、焦点回菜单 trigger；再次确认删除：行消失，焦点回路径输入。Mock 重置可以恢复测试数据。
- 菜单 ArrowDown 打开聚焦第一项，Home/End、循环、禁用项跳过、Esc 关闭与 trigger 恢复。
- 过期游标：旧页仍有 200 行、写操作禁用、提示刷新；刷新后恢复第一页。
- 上传完成、重复目标冲突且未覆盖；下载正常完成；慢速任务 Cancel 后先显示传输中 / 正在取消，确认终态才显示已取消。
- 失败任务显示“本地磁盘空间不足”及 cleanupRequired 临时路径；取消文件选择不新增任务。
- AppShell 组合：文件与终端切换保留一个 xterm，回终端聚焦 Terminal input，传输在切换期间继续，状态栏计数为 1。
- 活动传输下断开按钮打开 Dialog，未勾选停止任务时 Confirm disabled；勾选后断开、文件操作禁用，poll 确认任务 cancelled，状态栏归零。
- Light / Dark，1440×920 与 860×640，document.scrollWidth 等于 viewport 宽度，无页面横向溢出；窄屏表格有自身滚动区。最终组合 Harness 无 error/warn 日志。

证据：`screenshots/phase-7/files-transfers-light.jpg`、`files-transfers-dark.jpg`、`workspace-dark-860.jpg`。

## 6. Tauri 实测

**未执行 GUI / 系统文件选择器手动操作**。按用户更新后的门禁，不作为本阶段阻塞项。生产仍通过现有 Tauri API、Core 文件 token 和 Transfer Manager；系统 picker、WebView 的文件操作键盘行为、实际窗口切换 / 关闭尚未现场核实。真实后端 SFTP 由上述 OpenSSH 集成测试验证。

## 7. 剩余风险

- Native picker / WebView GUI 未实测；Browser Mock 不替代该证据。
- 大文件压力矩阵未重跑；本轮没有修改 Rust 分块传输实现。
- Remote File View/Edit 仍不可用，属于既有后端缺口。
- Phase 6 验收缺口继续保留，不由本阶段测试消除。

## 8. 退出结果

**PASS（用户认可的自动化 + Browser 门禁）**。提交并公开推送 main 后进入 Phase 8；后续阶段若验收失败，仍停止并报告 BLOCKED。
