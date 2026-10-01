# Phase 11 验收报告：响应式 Browser QA

检查日期：2026-10-01（Asia/Shanghai）；基于 Phase 10 `171691b`。

## 根因与验收范围

本阶段补足中等和更大视口、同一活动终端的连续 resize 证据，没有发现需要修改产品代码的布局问题。用户已明确将后续阶段门禁改为 Browser 通过即可继续；因此阶段状态为 **PASS（用户授权的 Browser 范围）**。这不等于 macOS/Windows 完整真机 QA PASS。

Phase 6 保留原 BLOCKED 验收缺口，用户此前授权提交并进入后续阶段；本阶段不将其改写为通过。

## 修改内容与文件

- 增加 `docs/refactor/screenshots/phase-11/`：16 张代表场景 JPEG、两份 capture manifest、一份活动终端 resize 记录。
- 更新本报告、`frontend-v2/README.md`、`design-qa.md` Page 18。
- 产品代码、依赖/lockfile、Rust Core、IPC Contract、旧 frontend 与 Tauri 配置未改。

## 自动化检查

| 检查 | 结果 |
| --- | --- |
| 新增截图 manifest / SHA256 / 独立重复一致性 | PASS，16/16 |
| 实际 CSS viewport / DPR / theme / lang / font loading / 横向溢出 | PASS，16/16，DPR=1 |
| git diff --check | PASS |
| type-check / Vitest / build | 沿用同一产品源码 Phase 10 最新 PASS：125/125 tests；本阶段仅文档和证据，未重复执行 |
| Rust 检查 | 未重跑；Rust 未修改 |

## Browser 实测

CUA 内置 Browser，macOS，系统字体。Phase 10 已覆盖 860×640、1440×920 的全部 23 场景、双主题/双语言，本阶段补充：

| 条件 | 覆盖与结果 |
| --- | --- |
| 1080×760，Light / zh-CN | 新增、Terminal、Focus、Files、Transfer、Monitor、Terminal settings、Palette；8 张独立 reload 两次 byte-identical，无横向溢出 |
| 1920×1080，Dark / en | 同上 8 场景，独立重复一致，无横向溢出 |
| 同一活动 Terminal：860→1080→1440→1920 | Shell ready；51×20→74×26→110×33→167×41；sidebar 190→220→236→236px；host ID 不变，仅一个 xterm，固定 transcript 保留 |
| 专注模式再缩至 860×640 | 96×24；退出后回到 51×20；同一 host，仅一个 xterm，焦点恢复 Terminal input，无横向溢出 |

截图审阅包含中等视口的终端、文件、监控、新增和设置/Palette 弹窗，以及大视口的专注、传输与设置。弹窗控件与底部操作可见，文件列表和 Monitor 内容使用自身滚动容器。窗口交通灯所需顶部留白存在；Browser 不绘制或验证原生交通灯。

resize 使用真正 viewport 操作触发 ResizeObserver / FitAddon / Mock typed IPC；读取可见 DOM 中的 footer 与实例 ID，没有注入事件或调用内部 controller。这验证前端重排路径，不证明真实 PTY 或 Windows WebView2 的相同行为。

## Tauri 实测与剩余风险

- Phase 9 独立 macOS release 应用已由用户验证 SSH/终端、快捷键、菜单、设置、主题/语言、拖动、窄窗口及重启持久化；本轮用户确认“没有问题，继续”。
- 本阶段未补做原生四尺寸、最小化/恢复、Overlay/交通灯点击、系统 picker / Keychain 检查。
- 未获得 Windows 真机或 runner；WebView2、Credential Store、Windows 路径、窗口和安装包均未实测。不能用 Browser/macOS 或交叉编译代替 Windows QA。
- Phase 6 尚存手动行为验收缺口；远程 View/Edit 仍是既有 Core 能力缺失，保持禁用。
- 阶段 **PASS（Browser 门禁）**，完整双平台发布验收继续 **BLOCKED**。按用户持续授权提交推送 main 并进入 Phase 12；不执行可选 Phase 13 Rust 大文件清理。
