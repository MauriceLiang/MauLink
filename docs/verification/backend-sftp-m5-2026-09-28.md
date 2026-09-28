# M5 SFTP 文件管理与传输验收记录

日期：2026-09-28  
范围：M5-01 至 M5-05 的 Core、Tauri IPC 和工作区 Files 页面。

## 已实现

- 目录浏览使用分页 cursor，限制目录页、响应和 SFTP 帧大小；游标有并发上限、空闲过期和明确清理路径。
- `stat`、新建目录、重命名和删除按 lstat 类型验证；符号链接删除只删除链接，根目录不可删，非空目录拒绝删除，目标存在时不覆盖。
- 本地文件由系统选择器授权并交换一次性、用途绑定的 token；前端不获得本机路径读取权限。
- 上传与下载按 64 KiB 块流式处理，进度经 Tauri Channel 更新；每任务使用同目录临时文件并执行无覆盖发布。
- 目标冲突、发布不支持、发布结果未知、取消清理失败和本机磁盘错误使用不同错误状态；传输活动时断开连接需要明确确认。
- Files 页面接入浏览、分页、路径跳转、上传/下载、建目录、重命名、删除、复制路径及传输进度/取消。

## 验证结果

| 检查 | 结果 | 证据 |
|---|---|---|
| Core 单元测试 | Passed | `cargo test -p maulink-core --lib --offline`：45 passed，1 ignored |
| OpenSSH SFTP 回环集成 | Passed | `cargo test -p maulink-core --test openssh_sftp openssh_sftp --offline -- --ignored --nocapture`：4 passed，1 filtered |
| 大文件往返矩阵 | Passed | `sftp_large_file_matrix_streams_with_bounded_chunks`：100 MiB、1 GiB、10 GiB 均进行上传/下载和 SHA256 对比；10 GiB 为 10,737,418,240 字节，往返用时 1,398.81 秒，整个测试函数用时 2,177.73 秒 |
| 发布冲突与磁盘错误分类 | Passed | 并发本机目标发布只有一个写入成功；`StorageFull`、权限错误及远端权限拒绝映射测试通过；发布冲突、能力不支持、结果未知分支各自保留不同语义 |
| Workspace 编译 | Passed | `cargo check --workspace --all-targets --offline` |
| 前端检查 | Passed | `node --check frontend/src/main.mjs`；`node --test frontend/tests/*.test.mjs`：6 passed；HTML/JS ID 静态对照无重复或缺失 |
| TypeScript 契约 | Passed | `cargo run -p maulink-core --example export_bindings --offline` 已导出 M5 DTO；Tauri capability 包含 Files 所需命令权限 |
| 实时 WebView IPC 性能测量 | Skipped by user choice | 未记录 WebView IPC 吞吐量或延迟 |

OpenSSH 集成覆盖 208 个目录条目的分页、中文/空格/引号/换行名称、符号链接和 cursor 限额；覆盖 stat、CRUD、远端只读权限拒绝、上传/下载校验和、一次性 token、已有目标冲突、非常规名称上传和取消不发布。大文件脚本使用固定大小块传输并流式计算 SHA256，没有把完整文件装入测试进程的单个缓冲区。

## 未验证范围

- 没有对真实满盘卷进行端到端故障注入；本机 `StorageFull` 分类由确定性单元测试覆盖。
- SSH 在远程 rename 请求发出后断开、服务端不支持安全发布、以及清理临时文件失败均未通过真实服务器故障注入；当前对这些结果的判定分支有单元测试，仍需部署目标服务器矩阵确认。
- 没有采集 10 GiB 运行时峰值 RSS；有界内存依据固定块实现和 10 GiB 校验往返验证，不报告峰值内存数字。
- 当前环境只验证 macOS；Windows 文件发布语义、凭据存储及桌面 UI 仍需平台验收。
- 用户选择跳过真实 WebView IPC 测量，因此不报告 IPC 延迟或吞吐量。
