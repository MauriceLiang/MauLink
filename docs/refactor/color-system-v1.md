# MauLink 颜色修正 v1.0 验收报告

日期：2026-10-01（Asia/Shanghai）。依据用户提供的《MauLink 颜色修正方案 v1.0》，只统一颜色系统，保持当前布局和业务行为。

**状态：PASS。本地自动化、Browser、macOS 构建及签名检查通过；用户已确认「配色没有问题了」，并授权更新 README、提交推送。**

## 修改内容与文件

- 建立 MauLink Blue / Slate Neutral 的语义颜色 Token；Light 与 Dark 各 27 个文档 Token 均精确匹配。系统深色分支与显式 Dark 使用相同值。
- 统一主按钮 normal / hover / active、轻量选中态、输入焦点、Checkbox / Radio / progress、原生 Select 的可样式化部分、菜单、Dialog、Toast、状态点与监控图表。
- 搜索外框承载 3px 淡蓝焦点环，内层输入框不重复加阴影；默认边框保持中性。
- 普通卡片保持 Surface，hover 使用 Border Hover，focus-within 使用 Primary Border；Sidebar 使用中性 Sidebar / Divider Token。
- 已连接使用 Success，旧指标使用 Warning，错误与删除使用 Danger；网络第二条曲线使用品牌蓝层级，避免把成功色当作装饰色。
- Toast 保持中性背景、边框和正文。现有组件没有状态图标，本轮不新增布局元素。
- xterm 读取共享 CSS Token，保持独立深色终端画布，移除紫色光标和选区；输入、ACK、尺寸和生命周期逻辑不变。
- DEV Harness 的 scoped 样式同步新变量名，避免旧变量移除后调试界面丢失颜色。
- 未引入 Reka UI；文档将组件迁移列为后续任务。保留既有 `base.css` 文件结构，无布局或依赖重构。

修改文件：

```text
README.md
frontend/visual/README.md
frontend/src/components/base/BaseContextMenu.vue
frontend/src/harness/ConnectionHarness.vue
frontend/src/harness/FilesHarness.vue
frontend/src/harness/MonitorHarness.vue
frontend/src/harness/ServerHarness.vue
frontend/src/harness/SettingsHarness.vue
frontend/src/harness/ShellHarness.vue
frontend/src/harness/TerminalHarness.vue
frontend/src/styles/base.css
frontend/src/styles/components.css
frontend/src/styles/connections.css
frontend/src/styles/files.css
frontend/src/styles/monitor.css
frontend/src/styles/servers.css
frontend/src/styles/settings.css
frontend/src/styles/shell.css
frontend/src/styles/terminal.css
frontend/src/styles/themes.css
frontend/src/styles/tokens.css
frontend/src/terminal/controller.ts
docs/refactor/color-system-v1.md
docs/refactor/screenshots/color-v1-review/*
```

## Token

Light 定义在 `frontend/src/styles/tokens.css`；Dark 与 System Dark 覆盖在 `themes.css`。

| Token | Light | Dark |
| --- | --- | --- |
| `--color-primary` | `#3B82F6` | `#60A5FA` |
| `--color-primary-hover` | `#2563EB` | `#3B82F6` |
| `--color-primary-active` | `#1D4ED8` | `#2563EB` |
| `--color-primary-soft` | `#EFF6FF` | `rgba(59, 130, 246, 0.12)` |
| `--color-primary-soft-hover` | `#DBEAFE` | `rgba(59, 130, 246, 0.18)` |
| `--color-primary-border` | `#BFDBFE` | `rgba(96, 165, 250, 0.35)` |
| `--color-bg-app` | `#FFFFFF` | `#111318` |
| `--color-bg-sidebar` | `#F8FAFC` | `#15181E` |
| `--color-bg-subtle` | `#F5F7FA` | `#191C23` |
| `--color-bg-hover` | `#F1F5F9` | `#20242D` |
| `--color-bg-active` | `#EFF6FF` | `rgba(59, 130, 246, 0.12)` |
| `--color-surface` | `#FFFFFF` | `#181B21` |
| `--color-surface-elevated` | `#FFFFFF` | `#1D2129` |
| `--color-text-primary` | `#172033` | `#EAECF0` |
| `--color-text-secondary` | `#667085` | `#A7AFBC` |
| `--color-text-tertiary` | `#98A2B3` | `#697386` |
| `--color-text-disabled` | `#B8C0CC` | `#4F5663` |
| `--color-text-on-primary` | `#FFFFFF` | `#FFFFFF` |
| `--color-border` | `#E4E7EC` | `#292E38` |
| `--color-border-hover` | `#D0D5DD` | `#363C48` |
| `--color-border-active` | `#93C5FD` | `#3B82F6` |
| `--color-divider` | `#EAECF0` | `#252A33` |
| `--color-success` | `#22A06B` | `#3CCB7F` |
| `--color-warning` | `#F59E0B` | `#F5B942` |
| `--color-danger` | `#EF4444` | `#F97066` |
| `--color-info` | `#3B82F6` | `#60A5FA` |
| `--color-focus-ring` | `rgba(59, 130, 246, 0.22)` | `rgba(96, 165, 250, 0.22)` |

补充 Token：`--color-overlay`、`--color-shadow`、`--color-danger-hover`、`--color-danger-soft`、`--color-text-on-danger` 及四个 `--color-terminal-*`。Danger hover / soft 使用现有语义 Token 的 color-mix；深色 Danger 按钮使用深色前景。终端色板独立于周围主题，避免切换主题改变远程内容的 ANSI 色语义。

## 移除的旧紫色与硬编码审查

已移除：`#5b5ce2`、`#4d4ed0`、`#a5a6ff`、`#eef0ff`、`#eeeeff`、`#252748`、`#292a50`、`#373785`。

`frontend/src` 中颜色字面值仅存在于 Token / theme 文件；50 个 CSS 变量引用均有定义。页面、组件及终端 controller 不再硬编码颜色。

保留：Logo / app icon 位图中的蓝青渐变；第三方 xterm 的默认 ANSI 色板和远程 ANSI 输出，属于终端内容语义。CSS 的 `transparent`、`currentColor`、`none` 是透明 / 继承 / 无填充行为，不属于独立色板。原生 Tooltip 与 Select 弹出面板由系统渲染，未新增可覆盖系统外观的组件。

## 自动化与 Browser

| 检查 | 结果 |
| --- | --- |
| TypeScript type-check | PASS |
| Vitest | PASS，11 files，128/128 tests |
| Vite build | PASS；现有 >500 kB chunk 提示保留 |
| 颜色字面值 / Token 引用审查 | PASS，旧紫色移除，变量完整 |
| 布局声明对比 | PASS，739 条既有尺寸、布局、间距、字体声明与修改前提交 44164e3 相同 |
| Browser 首页 / 添加表单布局前后对比 | PASS，1440×920 下位置、宽高、padding、gap、border width、字号相同 |
| 中文 Light / Dark，1440×920 | PASS，23 个场景 × 2 个主题，共 46 个组合 |
| 英文 Light / Dark，860×640 | PASS，首页、添加表单、终端、文件、外观设置、Palette，共 12 个组合 |
| 独立重复截图 / 字体 / 溢出 | PASS，58 个组合均两次独立 reload 后 byte-identical，无横向溢出，字体已加载 |
| 实际 Token 值 | PASS，Browser Light / Dark 分别与文档全部 27 项一致 |
| Search focus | PASS，边框 rgb(147,197,253)，rgba(59,130,246,0.22) 3px ring，内层 shadow none |
| 菜单危险项 | PASS，删除项 rgb(239,68,68)，默认背景透明 |
| System theme | PASS，当前系统 prefers-dark；保存 System 后 color-scheme dark、Primary #60A5FA |
| macOS arm64 release app | PASS，22.54 MiB，identifier io.maulink.desktop，版本 0.1.0 |
| ad-hoc 签名与严格校验 | PASS，codesign --verify --deep --strict |
| git diff --check | PASS |

58 组截图作为已确认配色的独立参考，保存在 `screenshots/color-v1-review/`，README 已使用其中的浅深首页及终端截图。Phase 10 / 11 / 12 的冻结基线保留为历史证据，未覆盖；新配色不与旧色板登记为像素一致。

## 交付与用户确认

应用：`/Users/mauriceliang/Documents/code/MauLink/target/release/bundle/macos/MauLink.app`。

最终签名后二进制 SHA256：`81eaf89958b02b9fd367168c2ac4210b7d7ecf11c9ce4445b34d768f13a42fc6`。

用户已过目 app 并确认配色没有问题。本轮确认范围为配色；原生 Select / Tooltip 的完整行为、Windows、原有 Phase 6 / 平台发行缺口仍保留。

按用户本轮约束，在 app 配色确认后更新 README，并将颜色修正、截图与报告纳入同一次中文提交。未声明完整平台或可访问性验收完成。
