# Phase 8 验收报告：共享 Monitor

检查日期：2026-10-01（Asia/Shanghai）

状态：**PASS（按用户更新后的 Browser 门禁）**。type-check、104/104 tests、build、Browser 质量矩阵与 Core Monitor 测试通过。Tauri GUI、真实 Linux 成功指标与原生最小化/恢复未现场实测，不记为实测 PASS；用户已取消 Desktop 手动门禁。允许自动提交推送 main 并进入 Phase 9。

前置提交：Phase 7 `49b8949e675ef4b93f32ae54e331c8eecca5a422` 已推送 main。Phase 6 未验收项仍按其报告保留，不由本阶段改记为 PASS。

## 1. 根因与目标

新工作区尚无 Quick / Full Monitor。读取 Core monitor.rs、Monitor Contracts、Tauri commands、旧 monitor-view.mjs 与单控制器逻辑、旧 Quick / Full 模板及 design-qa.md Page 11。复用既有快照、历史、活动同步与固定采集脚本，保持 Core 的前台/后台频率、backoff 和最小化生命周期。

首轮有两个回归断言失败：恢复后将隐藏的历史提示 DOM 当作可见告警；另一个计数混入已有连接轮询导致的宿主更新。分别改为可见告警检查，以及隔离连接计时、连续六次实际监控刷新后检查 xterm 宿主渲染次数。未移除验收断言，替代验收通过。收尾补充首次 Home 活动同步后，旧 Shell 测试的命令白名单需要包含 workspace_set_activity；增加了 null/false payload 与仅一次同步的检查，仍严格验证没有 SSH start。只读 Shell Harness 保持只读，不启动活动同步。

## 2. 修改内容

- AppShell 持有一个 Monitor controller；Quick / Full 共用同一个 shallowRef Snapshot 与历史数组。卡片和图表没有独立 timer / IPC，不把监控快照绑定到整个工作区的渲染依赖。
- 一条读取循环，完成当前批次后间隔约 1 秒读取 Core 快照；历史约每 5 秒统一读取六个 metric，最近 2 分钟、每条最多 120 点。
- 手动 Refresh 调真实 monitor_refresh，并重读历史；pending 时禁用，避免与自动读取重叠。Core 负责重新采样和 baseline，前端不模拟采集。
- workspace_set_activity 串行同步，保证迟到的旧工作区同步不会成为最终状态。首次 Home 同步 null/false，维持原 Core scheduler；连接、视图、专注切换更新真实活动。
- 文件 / Home / 专注模式停止前端监控读取；回到 Quick / Full 恢复一条循环。切连接清空旧指标，世代检查丢弃迟到快照和历史，断开保留最后已确认快照并标记为旧数据。
- 五类质量状态 ok / warmingUp / stale / unsupported / error 均有明确文案；unsupported 不显示 0，warmingUp 不冒充已采样，stale 有标签和淡化走势图。
- IPC 获取失败保留数据并显示“上次快照，并非实时数据”，原正常质量显示为 stale；历史单项失败保留旧图表并逐项提示，包括发送速率，恢复后撤除可见告警。
- CPU、内存、磁盘、网络、负载、uptime、系统信息与有界 SVG 走势；沿用二进制单位 KiB / MiB / GiB。Core decimal 字符串保持原样，格式化显示采用近似值。
- 终端侧边 Quick Monitor 和完整监控视图保留原 xterm；回终端聚焦输入。图表仅随历史数组变化计算，快照更新不重建终端。
- Tauri/Rust 的 Focused → is_minimized → set_window_minimized 路径未修改，也未添加前端替代最小化控制。Core 继续负责暂停、恢复 baseline、固定脚本、采集频率和历史上限。
- DEV-only Monitor Harness 的成功数值来自显式内存 Linux fixture；质量、快照/历史 IPC 失败、计数与主题可操作。既有 Files Workspace Harness 增加 unsupported 监控 fixture 以保持兼容，正式包排除 fixtures。

## 3. 修改文件

```text
frontend-v2/src/stores/monitor.ts
frontend-v2/src/monitor/view.ts
frontend-v2/src/components/monitor/{MonitorView,MonitorChart}.vue
frontend-v2/src/styles/monitor.css
frontend-v2/src/app/AppShell.vue
frontend-v2/src/components/terminal/TerminalWorkspace.vue
frontend-v2/src/i18n/errors.ts
frontend-v2/src/main.ts
frontend-v2/src/harness/{MonitorHarness.vue,monitor-fixtures.ts,files-fixtures.ts}
frontend-v2/tests/{monitor,shell}.test.ts
frontend-v2/README.md
docs/refactor/frontend-v2-phase-8.md
docs/refactor/screenshots/phase-8/*.jpg
```

无依赖变化。TypeScript 仍精确 5.9.3。Core、Contracts、旧 frontend、Tauri 配置和生命周期、package / lockfile 均无差异。

## 4. 自动化检查

| 检查 | 结果 |
| --- | --- |
| npm run type-check | PASS |
| npm run test | PASS，9 files、104/104 tests；本阶段新增 17 |
| npm run build | PASS，110 modules；HTML 0.42 kB、CSS 28.19 kB、JS 483.78 kB（gzip 134.64 kB） |
| cargo test -p maulink-core monitor::tests --locked | PASS，11 tests；其余 filtered 不计通过 |
| cargo test -p maulink-core --test openssh_sftp openssh_monitor_exec_runs_alongside_terminal_and_transfer -- --ignored --exact | PASS，1 passed / 5 filtered，实际隔离 OpenSSH 服务 |
| git diff --check | PASS |
| Core / 旧前端 / Contract / Tauri / 依赖无差异 | PASS |
| 正式 bundle 不含 Linux fixture 与 DEV 控件 | PASS |

前端测试保护：一条循环、五秒历史批次、120 点边界、初始 Home 同步、隐藏停读、恢复 / 释放、连接迟到响应、串行活动更新、Refresh 不重叠、快照失败与恢复、不支持指标不取历史、逐项历史错误、断开后旧数据、五种质量与 Quick/Full 一致、有效零值与缺失值、单位和 uptime、监控更新不带动 xterm 宿主。

Core 11 项覆盖 Linux / POSIX fixtures、固定脚本、不支持指标、CPU / 网络差分重置、采集频率、并发抑制与 backoff、有界历史。真实并发测试在本机 macOS 上采集了 system / disk，CPU / memory / network / load / uptime 正确为 unsupported，且并行终端和 64 MiB 上传完成。**没有据此宣称真实 Linux 所有指标已通过。**

既有 jsdom Canvas、Node localStorage 和未来 Vite config loader 提示为非阻塞信息；未引入无关依赖。

## 5. Browser 实测：PASS

Codex 内置浏览器，真实 click / select / check / press。`?harness=monitor` 使用生产 AppShell、真实 xterm 渲染器及 DEV Mock IPC；终端/指标不是实际 SSH 连接。

- Web-01 → 连接 → Quick Monitor；CPU 24.5%、内存 50.0%、磁盘 50.0%、网络 1.0 MiB/s / 256.0 KiB/s、负载 0.42，Full 对应数值一致；同一个 xterm 持续存在。
- 打开完整监控不创建另一份采集循环，手动刷新计数递增，历史统一刷新。
- warmingUp 显示正在采样；unsupported 显示不支持且无图表；error 显示暂不可用；stale 保留 24.5% 并显示数据已过期、淡化历史；恢复 ok 后显示正常。Quick / Full 对应状态与值一致。
- 快照 / Refresh IPC 失败：安全 timeout 文案、上次快照声明、stale 标识；恢复后撤除告警。
- 历史 IPC 失败：六项全部提示历史可能过期，恢复后没有可见告警。
- 回终端聚焦 Terminal input；文件视图 activity=false，连续文件 Dialog 打开 / 取消期间 snapshot/history 计数保持 359/480；恢复终端后递增。
- 进入专注 mode activity=false，退出 activity=true 并恢复输入焦点；xterm 数量始终 1。此项是专注模式验收，不冒充原生窗口最小化验收。
- 1440×920 Light / Dark，860×640 Dark；scrollWidth 等于 viewport 宽度。窄屏 Full 为两列并使用自身滚动，Quick 为 190px 滚动侧栏，终端可用宽度 480px，无页面横向溢出。

证据：`full-monitor-light.jpg`、`full-monitor-stale-dark.jpg`、`full-monitor-dark-860.jpg`、`quick-monitor-dark-860.jpg`。

## 6. Tauri 实测

**未执行 GUI 和原生窗口最小化/恢复操作**。用户取消 Desktop 手动门禁，故不阻塞 Browser PASS，但不记为实测通过。既有生命周期代码保持无差异，Core foreground/background/恢复 baseline 语义已读取，采集调度由 Core 测试验证；现场窗口行为仍未验证。

## 7. 剩余风险

- 真正连接 Linux 的成功指标没有现场 UI 实测；成功数值态为显式 Browser fixture，Core Linux 解析由既有测试覆盖。
- Native GUI / 最小化恢复未实测；没有修改或替代原生命周期。
- 原有连接/终端状态轮询仍可能使其宿主更新，本阶段仅验证监控更新没有新增此重渲染依赖。
- Phase 6 Browser Ctrl+C、突发输出等缺口继续保留。

## 8. 退出结果

**PASS（自动化 + Browser 门禁）**。提交并推送 main 后进入 Phase 9；后续阶段若验收失败，仍停止并报告 BLOCKED。
