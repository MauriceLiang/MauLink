# 凭据编辑简化与查看保护

## 范围

本改造包含两部分：已保存凭据在编辑服务器时默认保留、按需更换/移除；已保存登录密码或私钥口令的单条查看需要应用级安全策略授权。初始策略为 `deny`。私钥文件内容和路径不属于可查看凭据。

凭据查看保护只控制 MauLink 界面是否展示已保存凭据，不改变凭据的保存加密方式，也不影响 SSH 自动认证。二级密码不是数据库密钥、SSH 密码或主加密密钥。本功能不抵御拥有当前用户文件与进程访问权限的恶意软件，也不保证 WebView 中的 JavaScript 字符串能从进程内存物理清除。

## 结构与安全边界

- `CredentialRevealService` 位于 `maulink-core`，是查看策略、Argon2id 校验、失败锁定和凭据读取的唯一授权边界。Tauri handler 只转发 typed IPC 请求。
- 策略存于独立的 `credential_reveal_policy` 表；迁移 `0007_credential_reveal_policy.sql` 为旧数据库和新数据库写入 `deny`。该迁移不修改或删除服务器及既有凭据。
- 安全策略不进入通用 `AppSettings`。公开策略 DTO 不含密码哈希、盐或凭据明文。
- `protected` 每次查看单条凭据都在同一后端操作中校验二级密码，再读取当前服务器凭据；`direct` 也必须由后端当前策略明确放行。`deny`、错误密码、锁定和策略数据异常均不会读取凭据。
- 普通连接仍调用原 `CredentialManager::load_for_authentication()` 流程；WebView 没有直接读取命令。
- macOS 使用 `LocalAuthentication` 的 `deviceOwnerAuthentication`，系统可使用 Touch ID、Apple Watch 或本机密码；`src-tauri/Info.plist` 声明身份验证用途。Windows 使用 `UserConsentVerifier` 的 HWND interop API，并只接受 `Verified`。它表示系统设备身份验证/同意结果，不代表管理员权限；桌面 interop 文档列出的最低客户端版本是 Windows Build 22000。不可用或 API 调用失败时拒绝策略提升。参考 [Apple LAContext](https://developer.apple.com/documentation/localauthentication/lacontext)、[Apple deviceOwnerAuthentication](https://developer.apple.com/documentation/localauthentication/lapolicy/deviceownerauthentication)、[Microsoft UserConsentVerifier](https://learn.microsoft.com/en-us/uwp/api/windows.security.credentials.ui.userconsentverifier) 和 [RequestVerificationForWindowAsync](https://learn.microsoft.com/en-us/windows/win32/api/userconsentverifierinterop/nf-userconsentverifierinterop-iuserconsentverifierinterop-requestverificationforwindowasync)。
- Argon2id 使用 PHC 格式、64 MiB 内存、3 次迭代、1 个 lane；二级密码长度为 12–128 个字符，后端限制 UTF-8 不超过 1024 字节。连续 5 次失败会持久锁定 30 秒。KDF 在 `spawn_blocking` 中执行，服务操作锁限制同一实例内并发安全操作。

## 界面行为

- 设置页提供 `deny`、`protected`、`direct` 三种互斥策略，独立加载和保存。
- 开启或更改允许明文查看的策略时，Rust 主动调用系统身份验证；前端不能提交一个“已验证”布尔值。`direct` 还要求两项独立风险确认和确认短语；从 `protected` 降级到 `direct` 还要验证当前二级密码。
- 已保存凭据只从服务器编辑页的“更多凭据操作”菜单主动查看，不自动显示，也不写入新密码草稿。
- 查看结果只存在独立弹窗的短时组件状态中；15 秒、窗口失焦、页面隐藏、关闭、服务器切换和组件卸载都会清除可控引用。请求晚到时不会重新显示旧服务器的结果。
- 开发 Harness 只使用标记为 fixture 的假策略和假凭据，不访问系统凭据库或正式数据目录。

## 实施阶段记录

| 阶段 | 结果 | 说明 |
| --- | --- | --- |
| P0 | 完成 | 基线与原生认证可行性记录见 [P0 基线](./P0-baseline.md)。 |
| P1–P2 | 完成 | 当前分支基线提交 `ecbd3c6` 已实现凭据编辑默认保留与按需操作；本次保留现有身份字段校验。 |
| P3 | 完成 | 独立 deny 策略、Argon2id、持久限流、revision 和失败关闭路径。 |
| P4 | 部分完成 | macOS 与 Windows 原生适配器已实现并纳入生产命令。macOS 原生授权成功/取消未实机触发；Windows 交叉检查因缺少 Windows SDK `windows.h` 未完成，且没有 Windows 桌面实机验证。 |
| P5 | 完成 | 明文只经后端受策略保护的单条读取接口；未授权路径有计数器测试。 |
| P6 | 完成 | 安全设置、查看弹窗及 deny/protected/direct 交互已接入；Web Harness 已验收 deny、启用保护、假凭据查看和失焦后清除。 |
| P7 | 部分完成 | workspace 与前端自动化、Harness Web 流程已通过；macOS 系统身份验证真实成功/取消及 Windows 实机矩阵尚未验证。Windows 交叉检查也受本机缺少 Windows SDK 阻塞。 |
| P8 | 完成（平台限制已记录） | README 与本文件已更新；格式、lint、workspace/frontend 回归和视觉脚本均已执行。macOS app bundle 按交付流程在 source commit 推送后重建，构建结果由本次交付记录给出。 |

## 实机验收限制

仅在实际 macOS 和 Windows Tauri 应用中分别验证原生系统身份验证成功、取消、不可用/失败、焦点变化和重启后的策略持久化后，才能将 P4/P7 标记为完整通过。Linux 等不支持的目标保持 `deny`。开发 Harness 结果不等同于原生 API 实机结果。

本地用户具有同等系统权限时，仍可能检查或篡改应用数据和进程；二级密码不提供数据库策略防篡改、SSH 凭据二次加密或应用进程隔离保证。
