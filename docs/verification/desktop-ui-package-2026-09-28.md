# MauLink 桌面 UI 与本地打包记录

**日期：** 2026-09-28  
**状态：** macOS arm64 `.app` 与 ZIP 已于 2026-09-28 16:23 按当前源码重建并通过静态包检查；截图视觉验收、桌面交互验收、DMG 和跨平台发布验收未完成。

## 本轮结果

- 正式窗口继续使用本地 `frontend/` 页面，包含服务器管理、终端、SFTP 文件管理和服务器监控工作区。
- 前端采用 UI 原型的浅灰/靛蓝桌面布局，覆盖空首页、服务器侧栏、Terminal/Files/Monitor 工作区、设置主题卡和现有连接确认弹窗；快速监控展示 CPU 历史、内存/磁盘占用、网络速率和系统负载。
- 增加设置页面：用户可保存断开连接前确认、终端字体、字号、光标样式和滚动缓冲区；修改会应用到现有终端，且隐藏终端不会因设置变更而触发布局重算。
- 设置页通过已注册的 `settings_get` / `settings_update` command 读写版本化的本机设置记录。
- 主题偏好已接入系统/浅色/深色样式；英文偏好可保存，但当前页面文本仍以简体中文为主。
- 按用户提供的当前页面截图和添加服务器原型修正导航栏的重复 macOS 红黄绿圆点；添加服务器弹窗改为紧凑布局、认证分段切换、密码显隐按钮和折叠高级设置，同时保留单独保存、测试连接、保存并连接与凭据管理操作。
- 真实 Tauri IPC 性能测量按用户选择跳过。

## 构建产物

使用 Tauri CLI 2.12.0 和 Rust stable，在 macOS arm64 上执行：

```bash
PATH=/private/tmp/maulink-tauri-tools/bin:$PATH cargo tauri build --bundles app --no-sign -- --locked
ditto -c -k --sequesterRsrc --keepParent \
  target/release/bundle/macos/MauLink.app \
  target/release/bundle/macos/MauLink_0.1.0_aarch64.zip
```

生成文件：

- `target/release/bundle/macos/MauLink.app`（约 19.6 MiB）
- `target/release/bundle/macos/MauLink_0.1.0_aarch64.zip`（6,582,077 bytes）

ZIP SHA-256：`ca50020e70d78b4f679fc3c53c160a54f5506f3b2c10aa919c206bd17ffea70f`

## 已验证

- `cargo tauri build --bundles app --no-sign -- --locked` 成功；重建时间为 2026-09-28 16:23（本机时区），Release 编译约 26 秒。
- `file` / `lipo -info` 确认应用二进制是 Mach-O arm64；`Info.plist` 的 bundle ID 为 `io.maulink.desktop`、版本为 `0.1.0`。
- `unzip -t` 检查 ZIP 完整，无压缩数据错误。
- `cargo test -p maulink-core --lib --offline`：57 项通过，1 项原生凭据测试按设计忽略。
- `cargo check --workspace --all-targets --offline` 和 `cargo fmt --all -- --check` 通过。
- `node --check frontend/src/main.mjs`、`node --check frontend/src/icons.mjs` 与 `node --test frontend/tests/*.test.mjs` 通过，9 项测试全绿；新增的 `eye` / `eye-off` 图标可正常生成 SVG。
- 页面静态检查通过：HTML ID 唯一、label 目标可解析，更新后的弹窗元素 ID 存在。
- `codesign` 显示应用为本机 ad-hoc 签名，没有 Developer ID 和公证信息。

## 尚未验收与限制

- 本轮根据用户提供的当前页面截图和原型截图修正了导航栏与添加服务器弹窗，但没有启动桌面窗口获取修正后的截图，因此最终像素级视觉验收和真实 UI / SSH / SFTP 端到端验收仍未完成。浏览器策略拒绝本地 `file://` 预览，本地预览服务器也无法绑定回环端口；详见 `design-qa.md`。
- Windows 安装包与 Windows 真机验收未完成。
- DMG 在 2026-09-27 的构建中因系统 `hdiutil` / DiskManagement 环境错误失败；本轮只重建 `.app` 与 ZIP。
- 英文界面翻译尚未完成，主题偏好虽已接入，仍未进行运行时截图验收。
- 当前本地包适合开发和内部试用；面向公众发布前仍需 Developer ID 签名、公证、DMG 以及目标系统安装验收。
