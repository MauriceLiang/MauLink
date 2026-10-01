# 应用图标和样式设置验收

日期：2026-10-01（Asia/Shanghai）。用户要求应用图标使用新 logo，设置支持浅色、深色两种样式。

## 实现与兼容

- 使用 Tauri CLI 2.12.0 从两张用户原图生成平台图标。默认 ICNS / ICO / PNG 使用 Light；运行时备有 Light / Dark 的 256×256 RGBA PNG。替换旧图标，移除不再使用的旧 M 标记 SVG；生成方法见 `src-tauri/icons/README.md`。
- 设置 → 外观新增“应用图标样式”，支持浅色、深色和图像预览。草稿只在保存后生效；取消不写入。此选项独立于界面主题，顶部 logo 仍按界面主题选择。
- `AppSettings.appIconStyle` 使用枚举 `light | dark`，由现有 SettingsService 和 settings_update IPC 保存到 SQLite，沿用 revision 冲突检查，不增加 SQL schema migration 或新的 IPC 命令。
- 新字段在 Rust 使用 serde default。旧设置 JSON / 旧调用缺字段时默认为 light；无效值（包括 system）拒绝。TypeScript 合同由现有 export_bindings 生成；新的 typed caller 显式传入字段。旧字段和 API version 1 保持原样。
- macOS 使用 AppKit NSApplication.applicationIconImage，在主线程更新运行中的 Dock 图标；启动时从 SettingsService 恢复。其他桌面平台使用 Tauri Window.set_icon 更新运行窗口图标。
- 保存完成但系统图标更新失败时，返回结构化 AppError，并明确显示“设置已保存，但应用图标更新失败”。不伪装成保存成功；可重新加载当前 revision 后重试或重启，不回滚已提交的设置。
- 安装包 / Finder 中的静态图标默认为浅色，不随此选项改写已签名的安装包。新增说明和错误提示均有中英文。
- Rust 增加已在 lockfile 中的 objc2 / AppKit / Foundation 直接依赖，以及 Tauri image-png 功能所需的 image 等四个包；未升级已有锁定包。Vue、TypeScript 和 npm lockfile 未修改。

## 验证

| 检查 | 结果 |
| --- | --- |
| npm --prefix frontend run type-check | PASS |
| npm --prefix frontend run test | PASS，11 files，128/128 tests |
| npm --prefix frontend run build | PASS，128 modules |
| cargo check -p maulink-desktop --locked | PASS |
| cargo test -p maulink-core settings::tests --locked | PASS，3；旧数据默认值、非法样式、保存/冲突/重新加载 |
| cargo test --workspace --locked | PASS，70 passed，11 ignored；既有 SSH / 压力测试未运行 |
| cargo clippy --workspace --all-targets --locked -- -D warnings | PASS |
| cargo fmt --all -- --check / git diff --check | PASS |
| Browser 中文 / 英文，1280×720 / 860×640 | PASS；浅深预览、保存、重新打开、取消、独立主题；窄窗口无横向溢出，面板及保存按钮可见 |
| ICNS 解包检查 | PASS；256×256 代表图与 Light PNG 解码后的 RGBA 像素相同 |
| 正式 cargo-tauri build --bundles app --ci -- --locked | PASS，arm64，约 22.6 MiB |
| bundle 资源 / 内嵌运行图标 | PASS；ICNS 与新源文件一致，Light / Dark PNG 字节均存在于应用可执行文件中 |
| ad-hoc 签名 / codesign --verify --deep --strict | PASS；本地测试包，未公证 |

截图：`screenshots/logo/app-icon-dark-setting.jpg` 为浅色界面 + 深色图标；`app-icon-light-setting.jpg` 为深色界面 + 浅色图标，均在保存并重新打开设置后记录。

首次离线 cargo check 因缺少锁定依赖缓存而失败；按 lockfile 下载后通过。ICNS 解包 PNG 和独立 PNG 的编码字节不同，改为比较解码 RGBA，确认像素相同。没有改图像或降低视觉回归阈值。

## 产物与实际操作范围

路径：`/Users/mauriceliang/Documents/code/MauLink/target/release/bundle/macos/MauLink.app`。版本 0.1.0，identifier `io.maulink.desktop`。

可执行文件 SHA256：`b80e11bc13de4d4601a7a08980e13ccd201bafd8a6840637169b591472531fda`。

ICNS SHA256：`9c97e7d3d4b1018594b26c240b8432ced0b065f1405ac099a5b01fe027d72119`。

自动化、Browser 功能和打包检查 PASS。按用户既定分工，没有代用户操作原生应用；macOS Dock 实际切换/退出重启、Windows 真机效果尚未手动验证。请启动该产物，在设置中分别保存浅色与深色，检查 Dock；退出后重启，检查选择及 Dock 保留。切换界面主题不会覆盖图标选择。生产 chunk >500 kB 与 jsdom localStorage 既有提示保留。严格截图基线的既有环境限制沿用 frontend-logo-update.md，不宣称本轮全矩阵像素回归通过。
