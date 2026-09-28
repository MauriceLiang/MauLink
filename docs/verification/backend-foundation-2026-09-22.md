# MauLink 后端基础验证记录

**日期：** 2026-09-22  
**范围：** 后端实施文档 M1 基础工程  
**环境：** macOS 15.7.9，Apple Silicon，rustc 1.98.1，Node.js 24.20.0

## 已验证

| 检查 | 结果 | 证据 |
|---|---|---|
| Rust 格式 | Passed | `cargo fmt --all -- --check` |
| 全 workspace lint | Passed | `cargo clippy --workspace --all-targets --locked -- -D warnings` |
| 全 workspace 测试 | Passed | 7 passed，0 failed |
| 全 workspace 编译 | Passed | `cargo check --workspace --all-targets --locked` |
| 契约生成 | Passed | 生成 `contracts/v1` 中 6 个 TypeScript 类型 |
| Tauri 权限配置 | Passed | 编译期 schema 校验通过；main window 仅授予 `allow-app-get-info` |
| 依赖锁定 | Passed | 生成 `Cargo.lock`，Tauri 固定为 2.11.6 |

## 当前覆盖的业务行为

- `AppInfo` 使用 camelCase JSON 并报告 API version 1。
- `AppError` 输出稳定的 SCREAMING_SNAKE_CASE 错误码。
- 超过 JavaScript 安全整数范围的 `u64` 使用十进制字符串往返。
- 核心关闭会先取消并等待已登记任务；超过预算的任务会被中止。
- 关闭开始后拒绝登记新任务。

## 尚未验证

- Windows 编译和运行；当前机器未安装 Windows target，也没有 Windows runner。
- 实际启动 Tauri 调试窗口并点击调用；本轮仅完成编译期和单元测试验证。
- M0 的真实 OpenSSH、Host Key、PTY、SFTP、Keychain/Credential Manager 和 IPC 背压实验。
- SQLite、Server CRUD、Settings 与凭据一致性；这些属于后续 M2。

Docker 和 Podman 在当前开发机不可用。后续真实协议测试需要隔离的 OpenSSH fixture 或测试虚拟机，不能复用用户生产服务器和凭据。

