# 应用图标圆角及 Finder 同步检查

日期：2026-10-01。用户反馈安装图标直角，以及样式切换只改变 Dock、不改变 Finder 的应用文件图标。

## 圆角修改

- 新增 `tools/generate-app-icons.mjs`，通过 SVG clipping path 包装用户原图，再由 Tauri CLI 生成平台图标；原始两张照片保持不变。
- 1024×1024 画布使用 64 px 透明留白，896×896 背景和 200 px 圆角。所有 Light 系统 PNG / ICNS / ICO、Dark 运行 PNG 均更新。
- 设置预览改用与原生 256×256 图标逐字节相同的圆角图片，真实透明角不依赖 CSS 模拟。
- 原生资源测试检查四角及圆角区域透明、内部背景不透明、浅深背景区分正确。

## 检查

- TypeScript type-check PASS。
- 前端 11 files，128/128 tests PASS。
- Rust desktop 资源测试 1/1 PASS；Core / IPC 设置合同未改变。
- `cargo clippy -p maulink-desktop --all-targets --locked -- -D warnings`、`cargo fmt --all -- --check` 和 `git diff --check` PASS。
- Browser 中浅/深预览正常，保存并重新打开设置后显示所选样式。截图保存在 `screenshots/logo/rounded-light-setting.jpg` 和 `rounded-dark-setting.jpg`。
- 正式 macOS arm64 release 构建 PASS，约 22.54 MiB；已确认新的 ICNS 与浅/深运行 PNG 包含于产物。
- ad-hoc 签名及 `codesign --verify --deep --strict` PASS。
- ICNS 解包后的 256×256 图像与运行 PNG 的 RGBA 完全一致；设置预览与运行 PNG 逐字节一致。
- 产物：`/Users/mauriceliang/Documents/code/MauLink/target/release/bundle/macos/MauLink.app`。
- 当前可执行文件 SHA256：`57066cdb44e8c53354a1e90b9e7bb62e4752502cdb2c8c1522c5763ae59f281e`。

## Finder 同步实验及确认范围

上一版只使用 NSApplication.applicationIconImage，因此只改变运行中的 Dock 图标；Finder 读取安装资源。

本轮试用 [Apple NSWorkspace.setIcon(_:forFile:options:)](https://developer.apple.com/documentation/appkit/nsworkspace/seticon(_:forfile:options:))，仅操作 `/private/tmp/maulink-finder-icon-check/MauLink.app` 的隔离副本。副本开始时严格签名校验通过；设置浅色/深色文件图标后，校验均失败：

```text
resource fork, Finder information, or similar detritus not allowed
```

API 返回成功，但 NSWorkspace 图像读回的 TIFF hash 在两轮间相同，也未能证明 Finder 样式即时更新。未放宽签名规则，未修改 Resources 后重新签名以冒充原有发行签名。

已撤回 Finder 接口和相关依赖 feature。用户确认采用 Finder 固定浅色圆角、设置继续切换 Dock 的方案，设置说明已明确此范围。

按确认后的范围 PASS。原生 Finder / Dock 的 GUI 效果及 Windows 未实测；已完成 Browser 与文件/签名检查。现有 Vite chunk 大小和 jsdom localStorage 提示仍存在。
