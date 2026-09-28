# M5-01 SFTP 目录浏览验证记录

日期：2026-09-27  
范围：SFTP subsystem、远程 POSIX 路径、有限目录 cursor、文件属性和 IPC 命令注册。

## 实现

- `russh-sftp 3.0.0` 经 SSH session channel 的 `sftp` subsystem 建立独立 raw session。
- 首次浏览对 `.` 或指定路径调用 `realpath`，再 `opendir → readdir → close`；不通过 Shell 命令枚举目录。
- 每页最多 200 项、JSON 响应最多 256 KiB；服务端单个 SFTP 帧限制为 256 KiB。
- cursor 绑定 connectionId 与目录句柄；每连接最多 4 个，30 秒闲置过期。显式关闭、连接断开和应用关闭会释放 cursor。
- 返回 UTF-8 名称、POSIX 路径、类型、64-bit 字节数、修改时间、符号链接状态和权限。
- IPC 提供 `sftp_list_start`、`sftp_list_next`、`sftp_list_close`，能力标记启用 SFTP。

## 当日验证

当日记录为 40 项 Core 测试通过、1 项凭据库测试忽略；SFTP 浏览回环集成、Workspace 编译、Tauri 权限和 TypeScript 契约导出通过。非法字节文件名在 macOS 临时文件系统上无法创建，因此用单元测试验证 fail-closed 行为。

此记录只描述 M5-01 当日状态；M5 完整验收见 [M5 SFTP 验收记录](./backend-sftp-m5-2026-09-28.md)。
