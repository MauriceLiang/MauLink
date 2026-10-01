# 应用图标

原图保存在 `frontend/src/assets/maulink-logo-light.png` 和 `maulink-logo-dark.png`，保持原样。系统图标通过 SVG clipping path 添加真实透明圆角及留白：1024×1024 画布，背景区域 (64,64,896,896)，圆角半径 200。

使用 Tauri CLI 2.12.0 重新生成：

```sh
node tools/generate-app-icons.mjs
# 若 CLI 安装在临时目录，可传入完整路径：
node tools/generate-app-icons.mjs /private/tmp/maulink-tauri-tools/bin/cargo-tauri
```

脚本仅生成当前桌面项目所需的顶层 PNG / ICNS / ICO，将 Dark 的 256×256 PNG 保存为 `app-icon-dark.png`，并为前端设置预览同步两张真实圆角 PNG；临时 SVG / iOS / Android 输出自动清理。

安装包默认为浅色圆角图标；设置中的样式保存在 SettingsService，macOS Dock 和 Windows 运行窗口图标在保存后更新，启动时恢复。Finder 图标使用静态安装资源。动态 Finder 自定义图标接口在本机签名验证中失败，相关限制见 `docs/refactor/app-icon-rounded.md`。
