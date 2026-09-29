# MauLink UI 精修检查

**最终验收状态：blocked。** 前端改造、自动化检查和 macOS Release 构建已完成；本轮没有启动桌面窗口，因此尚无实现截图可与设计稿逐屏比对，不能确认像素级视觉验收。

## 对照材料

- `UI/index.html`：可交互中文原型，包含浅色/深色主题与连接、文件、监控、设置等状态。
- `UI/MauLink_UI_描述文档_v0.1.md`：双语界面、主题 token、桌面尺寸、信息层级和状态规范。
- `UI/slides.pdf`：73 页中文与英文设计画面，作为页面外观对照。

## 本轮改动

- 工作区按原型收敛为 46 px 标题区、38 px 终端标签栏、268 px 快速监控栏和 250 px 文件区；选中终端标签改为强调色底，不再使用下划线状态。
- 添加服务器弹窗调整到原型的 560 px 宽度、16 px 标题、38 px 输入控件；认证切换区和深色主题共用主题 token。
- 将深色主题色值对齐设计文档，补齐弹窗、表单、文件区、监控卡片和连接错误状态的暗色表面与文字颜色。
- 连接错误详情改为默认折叠，并添加静态契约测试，确保用户主动展开后才看到原始诊断信息。

## 设计源差异与未验收项

- 可交互 HTML 的浅色终端 token 为 `#FBFBFD`，但 PDF 的浅色工作区画面将终端展示为黑底。本实现保留 PDF 中的黑底终端外观；如以 HTML token 为最终规范，这一处仍需统一设计源。
- PDF 原型展示了模拟的进程/线程列表，设计说明限定快速监控指标，当前后端也没有进程列表数据接口；本实现没有把模拟数据带入真实界面。
- Inter 与 JetBrains Mono 字体依赖远程 Google Fonts，而桌面 CSP 不允许远程字体；应用继续使用本机系统字体回退，字形宽度可能略有差异。
- 由于没有本轮应用窗口截图，弹窗换行、原生 macOS 红绿灯对齐、桌面密度及中文/英文画面还需在实际运行窗口验收。

## 检查结果

- `node --check frontend/src/main.mjs`、`node --check frontend/src/i18n.mjs`：通过。
- `node --test frontend/tests/*.test.mjs`：15 项通过，0 项失败。
- CSS 语法解析器 `tinycss2` 未安装，完整解析检查未能运行；CSS 括号/引号平衡检查通过。
- `cargo build --release --package maulink-desktop --locked --offline`：通过。
- macOS `.app` ad-hoc 签名验证：通过；`Info.plist` lint：通过。
- `git diff --check`：通过。

构建位置：[`target/release/bundle/macos/MauLink.app`](/Users/mauriceliang/Documents/code/MauLink/target/release/bundle/macos/MauLink.app)
