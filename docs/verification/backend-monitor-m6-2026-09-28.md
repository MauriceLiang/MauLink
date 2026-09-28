# M6 Monitor 验收记录

日期：2026-09-28  
范围：M6 Monitor Core、Tauri IPC、工作区 Monitor 页面。

**状态：实现完成，可用环境内的检查通过；M6-05 的 Linux 实机指标比对仍待完成，当前不登记为完整验收通过。**

## 已实现

- 通过独立 SSH exec channel 运行固定采集脚本；脚本不接受前端命令或服务器返回的可执行内容。限制 exec 时长为 3 秒，并限制 stdout 与 stderr 合计为 256 KiB。
- 解析 CPU、内存、根文件系统、网络、负载、运行时间与系统信息。计数器无效或接口变化时重置速率基线；使用采样中点之间的单调时间计算网络速率。
- 每项指标独立返回 `ok`、`warmingUp`、`stale`、`unsupported` 或 `error` 状态。历史仅保留最近 15 分钟、每项最多 900 点；当前六项历史的理论上限低于每连接 2 MiB。
- 实现前台/后台不同采样频率、窗口最小化暂停、恢复时重置基线、跳过重叠采集及 5/15/30 秒失败退避。
- 增加 `monitor_get_snapshot`、`monitor_get_history`、`monitor_refresh` 和 `workspace_set_activity`；接入命令权限、能力声明、TypeScript DTO，以及窗口最小化状态桥接。
- 工作区增加 CPU、内存、主文件系统、网络、负载、运行时间和系统信息概览，并显示指标质量和近期趋势。
- 增加 Debian 12 与 Alpine 3 的采集文本 fixture，以及 BSD `df -kP` 附加 inode 列解析覆盖。

## 验证结果

| 检查 | 结果 | 证据 |
|---|---|---|
| Core 单元测试 | Passed | `cargo test -p maulink-core --lib --offline`：57 passed，1 ignored；覆盖解析、公式、历史上限、质量状态、调度间隔和退避 |
| OpenSSH Monitor/Terminal/Transfer 并发 | Passed | `openssh_monitor_exec_runs_alongside_terminal_and_transfer`：1 passed；本机 OpenSSH 上同时保持交互终端、64 MiB SFTP 上传并执行 Monitor refresh |
| OpenSSH SFTP 回归 | Passed | `cargo test -p maulink-core --test openssh_sftp openssh_sftp --offline -- --ignored --nocapture`：4 passed，2 filtered |
| Workspace 编译 | Passed | `cargo check --workspace --all-targets --offline` |
| Rust 格式检查 | Passed | `cargo fmt --all --check` |
| 前端检查 | Passed | `node --check frontend/src/main.mjs`、`node --check frontend/src/monitor-view.mjs`；`node --test frontend/tests/*.test.mjs`：9 passed |
| HTML/JS ID 对照 | Passed | 180 个唯一 HTML ID、161 个静态 JS ID selector，无缺失目标 |
| Tauri 权限与 TypeScript DTO | Passed | 工作区编译通过；`cargo run -p maulink-core --example export_bindings --offline` 导出 M6 契约 |
| 实时 WebView IPC 性能测量 | 按用户选择跳过 | 未记录 IPC 延迟或吞吐量 |

OpenSSH 并发验证运行在 macOS。Monitor 成功读取系统信息和根文件系统；Linux 专属 `/proc` 指标返回 `unsupported`，没有将缺失数据显示为零。该测试证明了 exec channel 可与 Terminal 和 SFTP 传输共存，不代表 Linux 指标已在 Linux 主机实测。

## 未完成验收范围

- M6-05 仍需在 Linux/OpenSSH 主机上对照 CPU、MemAvailable、Network、Load、Uptime 和 `df -kP /` 的真实输出，确认数值与质量状态；当前环境未提供 Linux 容器运行时或 Linux SSH 测试目标。
- 未在真实 MauLink WebView 中检查 Monitor 页面和 IPC 更新；用户选择跳过实时 IPC 性能测量。
- Windows 原生最小化/恢复事件和 Windows 桌面包尚未验收。
- P0 范围不包括 Disk IO、进程列表或非 Linux `/proc` 指标支持。
