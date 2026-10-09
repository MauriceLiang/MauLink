# 凭据编辑简化与查看保护：P0 基线

## 基线环境

- 审阅日期：2026-10-09
- Worktree：`/Users/mauriceliang/.codex/worktrees/0bd7/MauLink`
- 分支：`dev/app-updates`
- 基准提交：`addf5d458eb3c5dc0d26c2d0e3a42b2469230db3`（`重构：完善 SSH 私钥选择与凭据存储`）
- 本机：macOS 15.6，Apple Silicon，`rustc 1.98.1`，`cargo 1.98.1`
- 实施手册写的审阅提交 `2b2178126499d2b74e060a64bdd7c367a9262daa` 与本 worktree 当前 HEAD 不同；以下结论以当前 worktree 为准。

## 当前凭据读取边界

- `frontend/src/dialogs/ServerDialog.vue` 以 `CredentialUpdate` 的 `keep`、`replace`、`clear` 模式提交编辑；当前已有服务器页面通过下拉框要求选择其一。
- `frontend/src/dialogs/server-form.ts` 的 `credentialValidation()` 会在主机、端口、用户名、认证方式或私钥引用变化时阻止静默保留已保存凭据。
- `contracts/v1/CredentialUpdate.ts` 只传递凭据更新动作；`ServerProfile` 暴露 `hasSavedCredential`、`hasPrivateKey` 等状态，不返回密码、私钥口令或私钥路径。
- `crates/maulink-core/src/credentials.rs` 中 `CredentialManager::load_for_authentication()` 读取系统凭据库或 AES-GCM 加密数据库中的凭据。调用方是 Rust SSH 认证流程；当前没有对应 Tauri 明文读取命令。
- `src-tauri/capabilities/main.json` 没有查看凭据权限。代码库没有 `LocalAuthentication`、`LAContext`、`UserConsentVerifier` 或二级密码查看策略实现。
- 数据库最新迁移为 `0006_per_server_credential_storage.sql`，schema version 为 6。尚无隔离的凭据查看策略/限流状态表；不能把该策略放进通用 `AppSettings`。
- 私钥通过 `LocalFileRegistry` 的短时、用途绑定 token 供 Rust 解析，页面不接收私钥字节或真实路径。

## 原生身份验证可行性

- **macOS API 存在，仓库未集成。** Apple 的 `LAContext` 提供 `canEvaluatePolicy` 与 `evaluatePolicy`，`deviceOwnerAuthentication` 可使用 Touch ID、Apple Watch 或本机密码。当前 Rust 依赖中没有 Local Authentication 绑定；需要新增并审查原生桥接，再在签名桌面应用中实机验证成功、取消、不可用和失败行为。
- **Windows API 存在，桌面调用路径尚未接入。** Microsoft 文档说明 `UserConsentVerifier` 可检查并请求设备验证，但桌面应用需使用 `UserConsentVerifierInterop.RequestVerificationForWindowAsync` 并传入 HWND。当前 Tauri 命令/状态没有此适配器或窗口句柄流程。
- 本 worktree 在 macOS 上；本机不能证明 Windows 桌面窗口认证的集成、取消、策略禁用及无验证设备时均 fail closed。仅凭 API 文档不能作为 Windows 认证适配器已验证的证据。
- **P0 决定：** 明文查看维持默认禁止，不新增明文 IPC 或可放行查看的设置项。继续独立实施模块 A（P1/P2）；在两个平台适配器具备实现与实际验收证据前，不进入允许明文查看的 P4–P8。

原生 API 参考：

- Apple [`LAContext`](https://developer.apple.com/documentation/localauthentication/lacontext) 与 [`deviceOwnerAuthentication`](https://developer.apple.com/documentation/localauthentication/lapolicy/deviceownerauthentication)
- Microsoft [`UserConsentVerifier`](https://learn.microsoft.com/en-us/uwp/api/windows.security.credentials.ui.userconsentverifier)，其文档提供面向桌面应用的 HWND interop 路径

## 基线验证

- `npm test`（`frontend/`）：通过，19 个测试文件、207 项测试通过。Vitest 有现存的 JSDOM canvas/localStorage 提示，不影响本次基线结果。
- `cargo test -p maulink-core --locked`：92 项通过、1 项忽略；3 项 `preflight` 网络测试因当前执行环境禁止本机 TCP socket（`Operation not permitted`）失败。未修改测试或断言。
