# 应用图标

默认安装图标由 `frontend/src/assets/maulink-logo-light.png` 生成，深色运行图标由 `maulink-logo-dark.png` 生成。保留用户原图背景；不使用旧 SVG。

使用 Tauri CLI 2.12.0：

```sh
cargo tauri icon frontend/src/assets/maulink-logo-light.png --output /private/tmp/maulink-icons-light
cargo tauri icon frontend/src/assets/maulink-logo-dark.png --output /private/tmp/maulink-icons-dark --png 256
```

将 Light 输出目录的顶层 PNG / ICNS / ICO 复制至本目录，将 Dark 的 `256x256.png` 保存为 `app-icon-dark.png`。iOS / Android 子目录不属于当前桌面项目。

安装包图标默认为浅色；设置保存的样式在 macOS Dock、Windows 运行窗口图标上生效，启动时恢复。Finder 和未运行时的安装包图标保持默认样式。
