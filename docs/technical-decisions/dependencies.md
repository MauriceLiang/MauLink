# MauLink 后端依赖基线

本文记录 M0、M1、M2 数据层与 M3 SSH 连接层使用的直接依赖。`Cargo.lock` 是本次可重复构建的完整传递依赖清单。

## 工具链

- Rust：stable，当前开发机为 rustc 1.98.1，aarch64-apple-darwin。
- Edition：2024。
- 最低 Rust 版本：1.89。M3 使用的稳定版 `russh 0.63.3` 要求该版本；仍需在 CI 的最低版本任务中验证。
- Node.js：当前开发机为 24.20.0；M1 调试页面不需要 npm 依赖。

`rust-toolchain.toml` 使用 `stable`，因为当前环境只安装了 stable channel。发布候选前应在两平台确认工具链版本后改为具体版本号。

## M1 直接依赖

| 依赖 | Cargo 约束 | 首次锁定版本 | 用途 |
|---|---|---|---|
| tauri | =2.11.6 | 2.11.6 | 桌面宿主和 IPC；明确排除 3.0 alpha |
| tauri-build | 2.5 | 2.6.3 | 构建配置与应用 command manifest |
| tokio | 1.53 | 1.53.1 | 异步任务、有限等待与关闭 |
| tokio-util | 0.7 | 0.7.19 | CancellationToken |
| serde | 1.0 | 1.0.229 | IPC 序列化 |
| serde_json | 1.0 | 1.0.151 | 契约 fixture |
| thiserror | 2.0 | 2.0.20 | 结构化错误实现 |
| ts-rs | 12.0 | 12.0.1 | 从 Rust DTO 导出 TypeScript 类型 |
| uuid | 1.18 | 1.26.1 | 后续资源 ID；M1 先锁定序列化能力 |
| rusqlite | 0.38 | 0.38.0 | SQLite 单 worker、migration 与 backup API；启用 `bundled,backup` |
| tempfile | 3.23 | 3.27.0 | 在隔离临时目录运行真实 SQLite 组件测试，仅为开发依赖 |
| base64 | 0.22 | 0.22.1 | 编码不透明列表 cursor |
| sha2 | 0.10 | 0.10.9 | 将列表查询条件绑定到 cursor，不记录原查询内容 |
| keyring-core | 1.0 | 1.0.0 | 跨平台凭据窄接口与错误模型 |
| zeroize | 1.8 | 1.9.0 | Rust 自有 secret 缓冲在释放时尽力清零 |
| apple-native-keyring-store | 1.0 | 1.0.2 | macOS Keychain 原生实现，仅在 macOS 编译 |
| security-framework | 3.7 | 3.7.0 | 识别 macOS Keychain 授权错误码 |
| windows-native-keyring-store | 1.1 | 1.1.0 | Windows Credential Store 原生实现，仅在 Windows 编译 |
| tauri-plugin-dialog | 2.7 | 2.7.3 | 原生文件选择并转换为短期文件 token |
| russh | =0.63.3 | 0.63.3 | SSH 客户端；关闭默认 features，使用 `ring,rsa`，避免引入 AWS-LC 和未使用的压缩 |
| russh-sftp | =3.0.0 | 3.0.0 | SFTP raw session 与逐批目录枚举；与 russh 0.63.x 的 channel stream 配合 |

`rusqlite` 使用 bundled SQLite，避免开发机和 CI 的系统 SQLite 版本差异。平台凭据仅编译当前目标系统的原生 store，没有引入跨平台明文回退。M3 已加入 SSH 客户端依赖；M5 使用 `russh-sftp` 的 raw session 进行目录批次读取。该版本的客户端帧读取器按服务端声明的长度分配缓冲，因此 MauLink 在它外层加入 256 KiB 帧上限；该版本也会把非法 UTF-8 文件名变成 `�`，MauLink 遇到该字符时拒绝整个目录页，避免错误路径被操作。因为无法区分原始无效字节与合法 U+FFFD 字符，后者也会被保守拒绝。Windows 编译、发布包体积和后续 SFTP 传输仍需分别验证或评估。
