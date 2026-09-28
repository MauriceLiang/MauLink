# M3 SSH 实现与验收记录

**最终验收日期：** 2026-09-27  
**已验环境：** macOS Apple Silicon 本机运行、`x86_64-pc-windows-msvc` 全 workspace 交叉编译  
**结论：** M3 实现和规定完成标准全部通过，签署最终验收；M4 已开始开发。

## 已实现

- Connection registry 保存独立 connection ID、有限容量、严格状态转换、取消 token，以及绑定单次尝试的 Host Key/认证挑战。挑战默认 120 秒，迟到或重复响应失效。
- `SshConnectionManager` 校验已保存 Profile 的 `expectedRevision`；临时 draft 只允许 Test 模式，不写入 Profile 或 Credential 存储。同一 Profile 不允许重复连接，同时活动连接上限 20，同时建连/认证上限 4。
- 连接启动与 Profile 更新/删除共用操作锁。活动连接期间允许改名称和分组；连接参数、凭据、私钥路径变更及删除返回 `SERVER_IN_USE`。
- `SshConnector` 先解析地址，再执行 TCP/SSH 握手。网络预算来自 Profile 超时，Host Key 用户确认不消耗网络预算，每个阶段均可取消。
- 所有连接路径都经过 Host Key verifier。未知和变化的 key 都等待用户决定；保存时用旧 revision 做 CAS。拒绝变化 key 返回 `HOST_KEY_CHANGED`，Host certificate 当前明确拒绝。
- russh 配置移除 SHA-1 `ssh-rsa`，保留 RSA SHA-2；keepalive 使用 Profile 周期，连续 3 次未响应后终结连接。
- 密码与私钥认证只在 Host Key 验证后执行。缺少凭据时返回一次性挑战；密码、passphrase 和私钥文本缓冲尽力清零，不进入 connection snapshot。
- 错误区分错误凭据、未支持认证方式、认证超时、私钥读取/格式错误和错误 passphrase。
- `mode=test` 成功后立即断开；draft 的一次性密码/passphrase 只存在于当前尝试。快照返回 key exchange、host key、cipher、MAC 和 compression 协商信息。应用退出会取消尝试并清理活动 session。
- Tauri 暴露 `connection_start`、`connection_get`、`connection_cancel`、`host_key_respond`、`auth_respond` 和 `connection_disconnect`，并具备显式权限和生成的 TypeScript payload。

## 验收证据

最终 workspace 基线检查通过：

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `cargo check --workspace --all-targets --locked`
- `cargo test --workspace --locked`：core 33 项通过、1 项原生凭据存储测试按设计忽略；契约测试 3 项通过；真实 OpenSSH 测试默认 ignored，避免无 fixture 时空跑。
- `cargo run -p maulink-core --example export_bindings --locked`
- `cargo xwin check --workspace --all-targets --target x86_64-pc-windows-msvc --locked`

`openssh_connection` 使用仅绑定 `127.0.0.1` 的临时 OpenSSH fixture 验证并通过：首次保存与重连、`TrustOnce`、拒绝未知 key、变化 fingerprint 拒绝、Host Key 阶段取消、未监听端口、正确与错误 ED25519 key、正确与错误 passphrase、错误密码、已保存 Profile 与 draft Test Connection、协商算法和主动断开。fixture 不读取或写入用户 SSH 配置和凭据。

新增的自包含 `openssh_lifecycle` 测试通过：

```bash
cargo test -p maulink-core --test openssh_lifecycle --locked -- --ignored --nocapture
```

该测试在临时目录生成客户端 key 和两组服务端 host key，并在同一 loopback endpoint 上真实轮换 host key，验证：

- 首次 `TrustAndSave` 后，变化提示包含旧 fingerprint。
- 接受新 key 后 CAS 更新成功，再次重连不出现旧挑战或新挑战。
- 已认证连接遭服务端终止后进入 `Failed / CONNECTION_LOST`。
- 冻结服务端后，100 ms keepalive 连续超限，连接进入 `Failed / CONNECTION_LOST`。
- 隔离的 OpenSSH 进程组在成功和失败路径均被清理。

Host Key 拒绝测试曾发现取消竞态：状态机先记录 `HOST_KEY_REJECTED`，连接预算随后观察到取消并错误返回 `CANCELLED`。连接层现会先读取终态快照并保留原始错误；回归测试和真实 OpenSSH 场景均通过。

## 完成标准判定

| M3 完成标准 | 证据 | 结果 |
|---|---|---|
| 任何路径不绕过 Host Key | 未知、首次保存、TrustOnce、拒绝、真实轮换和保存后重连 | 通过 |
| Test mode 成功即关闭 | 已保存 Profile 与临时 draft 均进入 `Closed` 并释放占用 | 通过 |
| 错误端口、密码、passphrase、fingerprint 变化分别识别 | `CONNECTION_REFUSED`、`AUTH_FAILED`、`PASSPHRASE_INVALID`、`HOST_KEY_CHANGED` | 通过 |
| 重连不复用旧挑战 | challenge ID 单次有效；轮换接受后重连无残留挑战 | 通过 |
| 主动断开、网络断开和 keepalive 收口 | 主动关闭、服务端终止和冻结服务端测试 | 通过 |
| Windows 构建兼容 | Windows MSVC 全 workspace/all-targets 交叉编译 | 通过 |

Windows 交叉编译验证的是源码、条件编译、Tauri metadata 和链接前构建兼容性，不等同于 Windows 原生运行。Windows Credential Store、UTF-16 路径、系统网络栈和安装包仍需在 Windows runner/真机执行；它们属于 M8 双平台发布验收，不影响上述 M3 业务完成标准的签署。

## 依赖选择记录

- `russh 0.54.6` 因 yanked `libcrux-ml-kem 0.0.3` 无法由当前 crates.io 正常解析。
- `russh 0.57.1` 和 `0.61.0` 在当前 resolver 下解析到不兼容的预发布 RSA/ECDSA 依赖组合，未通过手工锁定整组预发布密码学 crate 绕过。
- 使用稳定版 `russh 0.63.3`，最低 Rust 为 1.89。关闭默认 features，仅启用 `ring,rsa`，并过滤 SHA-1 RSA 签名算法。
