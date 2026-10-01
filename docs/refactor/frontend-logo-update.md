# 品牌 logo 替换验收

日期：2026-10-01（Asia/Shanghai）。用户要求将 ZIP 中两张图片分别用于浅色、暗色模式，验收后提交推送，后续提交消息使用中文。

## 修改

- 两张原始 1254×1254 RGB PNG 保存至 `frontend/src/assets`，与 ZIP 内容逐字节一致，未改变像素、背景或颜色。
  - Light SHA256：`9365ccbda761fede7d6bc85a0db63e8c8f482e412b23f2b7aad001ccb4be4bd4`
  - Dark SHA256：`294f978575e92ab35a7fa8726583266a263fbfeb53e8dc99e0eb9d618c547672`
- TopBar 品牌按钮使用 32×32 图片替换原 M 标记。沿用现有 `data-theme` 和 `prefers-color-scheme`：light/dark 选择对应图片，system 跟随系统。
- 保留按钮名称、首页点击行为、焦点和窗口拖动边界。图片为空 alt 并 aria-hidden，避免重复朗读。
- 根 `AGENTS.md` 记录 Git 提交标题和正文使用简体中文的约定。

## 检查结果

| 检查 | 结果 |
| --- | --- |
| npm --prefix frontend run type-check | PASS |
| npm --prefix frontend run test | PASS，11 files，126/126 tests |
| npm --prefix frontend run build | PASS，128 modules，两张 PNG 正确输出 |
| 原有截图工具单元测试 | PASS，1 test；工具未修改 |
| 历史截图 manifest / SHA256 / 尺寸校验 | PASS；仅校验原有历史资料完整性，不代表当前 UI 的像素回归通过 |
| Browser 实际主题保存切换 | PASS，light → dark → system；两图已加载且恰有一个显示，32×32 |
| Browser 窄屏及可访问名称 | PASS，860×640 无横向溢出，名称仍为“MauLink 服务器工作台” |
| git diff --check | PASS |

实际效果保存在 `screenshots/logo/light.jpg` 与 `dark.jpg`。Browser 使用 DEV Mock，设置保存隔离于正式资料；末次 system 实测操作系统为深色，对应 Dark 图片正确显示。未主动修改操作系统外观或测试 OS 外观切换事件。

## 截图回归限制

本轮曾尝试批量更新 Phase 10 实现截图。虽然独立加载配对得到相等截图，随后默认基线回归仍在 8 个首页条件中有 7 个报 baseline changed，不能记为通过。

为定位原因，临时恢复 HEAD 原来的 M 图标与 CSS，并使用 HEAD 原始基线进行对照。未修改界面在 dark/en/860×640、light/zh-CN/1440×920 两个条件均报同样的 baseline changed；DOM 确认显示原 M 标记。这证明当前截图环境下，严格字节比较的失败也存在于未修改的旧界面。尚未确定具体渲染差异来源。

已撤销本轮批量基线及截图工具试改，保留 Phase 10/11/12 历史记录，不降低阈值、不把候选图认作通过。本次 logo 以自动化功能检查、构建及实际主题/布局检查验收；严格像素回归仍未通过，作为独立限制记录。

## 范围和风险

本次应用内顶部 logo 替换的功能验收 PASS。依赖、Rust Core、IPC 和 Tauri bundle icon 配置未修改；现有桌面 app 未重新打包，原生 WebView 本轮未实测。PNG 共约 1.67 MB，按用户原图保留。构建已有 >500 kB chunk 提示及 jsdom localStorage 提示仍存在。
