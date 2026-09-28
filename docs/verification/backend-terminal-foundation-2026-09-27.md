# M4 Terminal 开发与验收记录

**启动日期：** 2026-09-27  
**当前状态：** M4-01 至 M4-04 的核心实现和 OpenSSH 压力验证已完成；M4-05 调试页已实现。M4 最终验收仍待真实 Tauri IPC 指标记录。

## 已完成实现

- 每个 Terminal 在已认证 SSH connection 下建立独立 session channel，依次请求 `TERM=xterm-256color` PTY 与 Shell，并等待服务端确认。
- 提供 `opening / running / closing / closed / failed` 状态、尺寸、exit status、exit signal、结构化错误和时间戳快照。
- Terminal 资源限制为每连接 8 个、全局活动 32 个、历史快照 100 个。关闭一个 Terminal 只发送该 channel 的 EOF/Close。
- 输出保持原始字节语义，单 chunk 最大 32 KiB，以 Base64 跨 IPC；`seq` 和 `streamId` 使用累计 ACK。超前 ACK 与错误 stream 被拒绝，重复/旧 ACK 幂等。
- 每 Terminal 原始未 ACK 输出上限 128 KiB；应用层 pending 与 in-flight 原始输出合计上限 512 KiB。达到在途窗口时，发送任务仍可处理 ACK、取消和 SSH channel 读取，不会因 IPC channel 已满而卡住 ACK 超时处理。
- SSH channel receive window 为 256 KiB，russh channel 消息缓冲为 4 条，最大 packet 为 32 KiB。消费者 15 秒没有推进 ACK、接收端被丢弃或输出缓冲超限时，失败并关闭受影响的 Terminal channel。
- 输入单请求解码后最大 64 KiB，等待发送的输入预算 256 KiB。`inputSeq` 从 1 递增，跳号返回 `TERMINAL_INPUT_SEQUENCE_INVALID`，重试已接纳序号不会再次写入远端，满额返回 `INPUT_BACKPRESSURE`。
- 支持 Resize、显式 Close、应用退出清理和 SSH channel 结束后的终态收口。
- 新增 `terminal_open/get/write/resize/ack/close` Tauri commands、显式权限、capability，以及 Rust 生成的 TypeScript 契约。
- `tools/ipc-harness/index.html` 提供临时本地 IPC 调试消费者，可选择服务器配置并测量“收到即 ACK”与“UTF-8/ANSI 解析后 ACK”。它不是产品 UI。

## 验证结果

workspace 检查通过：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo check --workspace --all-targets --locked
cargo run -p maulink-core --example export_bindings --locked
```

真实 OpenSSH 双 PTY 与压力集成测试通过：

```bash
cargo test -p maulink-core --test openssh_terminal --locked -- --ignored --nocapture
```

覆盖同一 SSH connection 下多个独立 PTY、Resize、关闭一个 PTY 后另一 PTY 及 connection 仍可用、connection 断开后活动 Terminal 收口、错误 stream ACK 和跳号 inputSeq 被拒绝、重复 inputSeq 不重复执行、1 MB 输出在持续 ACK 时无损、停止 ACK 后只令受压 Terminal 进入 `TERMINAL_CONSUMER_STALLED` 且已发数据不超过 128 KiB、另一 Terminal 仍可响应，以及接收端丢弃后的有界失败。Rust 单元测试覆盖累计 ACK/窗口上限和 UTF-8/ANSI 字节跨 chunk 保真。

IPC harness 的 JavaScript 语法检查通过 `node --check`。Windows MSVC 全 workspace 交叉编译通过：

```bash
cargo xwin check --workspace --all-targets --target x86_64-pc-windows-msvc --locked
```

## 尚未完成的验收项

M4 最终验收仍要求在获准运行的桌面会话中，通过临时 IPC harness 记录下列真实数据：

- “收到 chunk 后立即 ACK”与“UTF-8/ANSI 解析后 ACK”两种模式的准确 payload 字节数、吞吐、首 chunk 延迟、平均 ACK 调度和往返时间。
- 测量结果、运行环境与可复现步骤。

本轮按执行指示跳过实时 IPC 测量，所以当前没有真实 Tauri IPC 数值；不能据此判断 Base64/JSON IPC 吞吐是否满足产品目标，也不将 M4 标记为最终验收通过。调试 harness 保留供后续获准测量使用。

应用层 512 KiB 是 pending 与 in-flight 的原始输出字节预算，不代表整个进程 RSS 上限；russh 接收队列、Base64/JSON 序列化副本和 WebView 消费队列不包含在该值内。
