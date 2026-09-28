# M2 数据层基础验证记录

**日期：** 2026-09-22  
**平台：** macOS Apple Silicon  
**范围：** M2-01～M2-05 当前实现

## 已实现

- `rusqlite` 单 worker 独占连接，调用队列容量为 32，队列满时返回 `STORAGE_BUSY`。
- 显式启用 `foreign_keys` 和 3 秒 `busy_timeout`，使用 `PRAGMA user_version` 管理 schema。
- 从版本 0 升级前通过 SQLite backup API 创建备份，最多保留 3 份；迁移在事务中执行。
- 较新 schema 返回 `SCHEMA_TOO_NEW`，迁移失败返回 `MIGRATION_FAILED`，不删除或重建原库。
- Group CRUD；删除 Group 后依靠外键规则将所属 Server 置为未分组。
- Server CRUD、字段校验、默认名称、参数化搜索、最多 200 条分页和 optimistic revision。
- Server 列表使用绑定查询条件与全局列表 revision 的 opaque cursor；条件不匹配返回校验错误，列表变更后旧 cursor 返回 `REVISION_CONFLICT` 并要求刷新。
- 私钥路径以平台编码字节保存在数据库内部，返回给调用方的 `ServerProfile` 只包含 `hasPrivateKey`。
- Settings 默认值、范围校验、整体 revision、持久化和数据库提交后的 `watch` 通知。
- TypeScript 契约中的 Unix 毫秒时间戳为 `number`，没有导出私钥路径字节。
- 原生文件选择返回 10 分钟有效、用途绑定的 UUID token；注册表最多 64 项，私钥文件限制为普通文件且不超过 16 MiB。
- 系统凭据通过容量 16 的独立 worker 串行访问；secret 的 `Debug` 始终脱敏，Rust 自有缓冲在释放时清零。
- 凭据替换使用 `pending_write -> active -> pending_delete`；旧项清理失败不破坏新活动凭据。
- `CredentialUpdate` 使用显式的 `Keep | Clear | Replace(secret)` tagged union；创建不接受 `Keep`，修改身份或私钥时不能用 `Keep` 隐式沿用旧凭据。
- `server_create` / `server_update` 只接收私钥文件 token。更新已有私钥 Profile 时可在可信原生层复用原路径，路径始终不返回 IPC。
- 删除 Server 支持保留凭据为 `retained`，或转为 `pending_delete` 后尝试清理；失败返回 `credentialCleanupPending=true`。
- Tauri 已接入应用数据目录、权限为 `0700` 的目录初始化、数据库、Settings、文件 token 与原生凭据 worker，并注册版本化的 Server/Group/Settings CRUD、文件选择和凭据清理 commands。

## 自动验证

以下命令均在依赖离线模式或不访问网络的情况下通过：

```text
cargo fmt --all -- --check
CARGO_NET_OFFLINE=true cargo clippy --workspace --all-targets --locked -- -D warnings
CARGO_NET_OFFLINE=true cargo test --workspace --locked
CARGO_NET_OFFLINE=true cargo check --workspace --all-targets --locked
CARGO_NET_OFFLINE=true cargo run -p maulink-core --example export_bindings --locked
```

测试结果：25 项通过，0 项失败，1 项原生凭据测试默认忽略。其中包括：

- 新库初始化与较新 schema 拒绝。
- 版本 0 数据库升级前备份。
- migration 冲突失败后 `user_version` 和原表结构保持不变。
- Group 删除不级联删除 Server。
- stale revision 返回 `REVISION_CONFLICT`。
- 数据库关闭并重新打开后 Server 和 Settings 仍可读取。
- 搜索大小写匹配以及 `%` 不被当作调用方注入的通配符。
- Settings 只在成功提交后发布变更。
- host、port、私钥认证配置和 Settings 范围校验。
- cursor 与查询条件、列表 revision 的绑定。
- 文件 token 的过期、用途、容量和路径不泄漏。
- 凭据替换失败清理、Server 删除时保留/删除凭据，以及待删除项显式重试。
- Server 创建的显式凭据动作，以及身份修改时 `Keep` 拒绝、`Clear`/`Replace` 的事务语义。
- Tauri workspace、command manifest 与 capability permissions 编译检查。

`CARGO_NET_OFFLINE=true cargo run -p maulink-desktop --bin maulink --locked` 已编译并进入 `target/debug/maulink` 事件循环，随后人工终止。当前桌面自动化环境无法枚举这个未打包调试进程的窗口，因此没有把按钮级 IPC 交互记为已验证；运行时仅观察到 macOS LaunchServices/辅助服务连接警告，进程没有因此退出。

本机还执行了以下真实 macOS Keychain 测试：

```text
CARGO_NET_OFFLINE=true cargo test -p maulink-core native_store_round_trip --locked -- --ignored
```

测试在写入阶段返回 macOS OSStatus `-60008`（当前进程无法获得该操作授权），因此未把真实 Keychain 读写删计为通过。该错误已映射为结构化 `CREDENTIAL_ACCESS_DENIED`，应用不会回退到明文存储。

## 尚未完成或未验证

- 连接管理尚未实现，因此“连接中只允许修改 name/group”的 `SERVER_IN_USE` 规则未接入。
- macOS 当前终端进程没有 Keychain 写入授权；需在已签名/已授权的桌面应用进程再次验证。Windows Credential Store 仍需在 Windows 主机完成真实读写删。
- 当前只在 macOS Apple Silicon 编译和运行测试；Windows 编译、Windows 非 Unicode 路径往返、bundled SQLite 包体积尚未验证。
