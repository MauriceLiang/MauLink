# MauLink UI 描述文档

**文档名称：** MauLink UI 描述文档  
**文档版本：** v0.1  
**产品名称：** MauLink  
**对应需求基线：** MauLink PRD v0.1 / MauLink MVP 设计文档 v0.1 / MauLink UX 设计文档 v0.1  
**设计阶段：** UI / Visual Design / Design System / 页面视觉规格  
**目标平台：** Windows、macOS  
**界面语言：** 简体中文、English  

---

# 1. 文档目的

本文档用于在现有 PRD、MVP 与 UX 设计基线之上，明确 MauLink MVP 的 UI 表现方式，使产品、设计、前端和 AI 设计工具对最终界面形成一致理解。

本文档重点定义：

- 整体视觉方向
- 颜色、字体、间距、圆角、阴影等基础视觉规范
- Application Shell 与 Server Workspace 的视觉结构
- Server Sidebar、Terminal、Files、Monitor 等核心组件的 UI 描述
- MVP 14 个核心页面 / 状态页的视觉要求
- Loading / Empty / Error / Disabled / Selected 等状态
- Windows / macOS 的一致性与差异
- 响应式桌面窗口行为
- 动效、性能感知与视觉稳定性要求
- 可访问性要求
- 可直接交付给 Figma、Pen.dev、前端 AI 或实现人员的页面描述基线

本文档不改变既有 PRD、MVP 和 UX 的功能边界。

> **重要：** 本文档中标记为“UI v0.1 推荐值”的颜色、字号、尺寸、圆角等属于 UI 阶段新增的推荐视觉参数，并非此前 PRD 已冻结参数；后续可通过设计评审和可用性测试调整。

---

# 2. UI 核心目标

MauLink 的 UI 不只是对功能进行“美化”，而属于 MVP 核心产品能力。

UI 必须同时满足：

```text
Modern
Simple
Professional
Clear
Fast
Stable
Restrained
```

对应中文体验目标：

- 现代
- 简洁
- 专业
- 清晰
- 高效
- 稳定
- 克制

最终用户感受应接近：

> “这是一款真正为开发者日常使用设计的现代桌面工具，而不是传统后台管理系统。”

以及：

> “我不需要学习 MauLink，打开以后自然就知道怎么用。”

---

# 3. UI 设计原则

## 3.1 Terminal First

Terminal 是 Server Workspace 中最高优先级的工作区域。

空间不足时优先级必须保持：

```text
Terminal
>
Server Navigation
>
Quick Monitor
>
Files
```

任何视觉设计都不得让 Monitor、Files、装饰性元素明显挤压 Terminal。

---

## 3.2 Workspace First

MauLink 不应设计成多个彼此割裂的独立工具页。

核心界面模型始终为：

```text
Server
↓
Workspace
├── Terminal
├── Files
└── Monitor
```

用户连接服务器后，所有主要功能围绕“当前服务器上下文”展开。

---

## 3.3 Progressive Disclosure

默认只展示完成当前任务所需的信息。

以下内容不得抢占默认界面：

- Jump Host
- Proxy
- Compression
- Algorithm
- Keep Alive 细节
- Startup Command
- Environment
- 高级 Monitor 指标

这些内容应进入：

```text
Advanced
More
Details
Detailed Monitor
```

---

## 3.4 Low Visual Noise

MauLink 必须避免传统运维后台常见的视觉噪音。

明确避免：

- 大量卡片堆叠
- 每个区域都加边框
- 大面积高饱和品牌色
- 大面积渐变
- 大面积毛玻璃
- 过重阴影
- 3D 图表
- 发光效果
- 长时间循环动画
- 无意义装饰性图形
- 每个指标都使用不同颜色
- 页面中同时出现过多 Primary Button

---

## 3.5 Stable Layout

动态数据刷新不得导致页面抖动。

特别是：

- CPU 9% → 100%
- Memory 900 MB → 12.4 GB
- Network 80 KB/s → 15.8 MB/s

数值区必须预留稳定宽度，Monitor 刷新不得推动相邻组件位移。

Files 刷新时保留当前路径和已有内容，不整块闪白。

---

# 4. 整体视觉方向

## 4.1 视觉关键词

MauLink 视觉设计使用以下关键词约束：

```text
Developer Tool
Desktop Native Feeling
Minimal
Neutral
Dense but Readable
Precise
Calm
Professional
```

不追求“炫”，而追求长时间使用不疲劳。

---

## 4.2 视觉气质

推荐采用：

> **中性灰为主体 + 低饱和单一强调色 + 清晰状态色。**

整体背景、Sidebar、Panel、Toolbar 之间主要依靠轻微明度差、间距和层级区分，而不是大量描边和阴影。

强调色只用于：

- Primary Action
- 当前选中项
- Focus Ring
- Active Tab
- 可交互链接
- 少量品牌识别

不得使用强调色大面积铺满 Sidebar、Header 或主工作区。

---

# 5. 主题系统

MVP 支持：

```text
System
Light
Dark
```

默认建议：

```text
System
```

切换主题后尽量即时生效。

## 5.1 Light Theme — UI v0.1 推荐值

| Token | 推荐值 | 用途 |
|---|---:|---|
| `bg.app` | `#F6F7F9` | 应用整体背景 |
| `bg.surface` | `#FFFFFF` | 主工作区 / 弹窗 |
| `bg.subtle` | `#F8F9FB` | Sidebar / 次级区域 |
| `bg.hover` | `#F0F2F5` | Hover |
| `bg.selected` | `#EEF0FF` | 选中项 |
| `border.default` | `#E5E7EB` | 必要分割线 |
| `border.strong` | `#D1D5DB` | 强分割线 |
| `text.primary` | `#171A21` | 主文字 |
| `text.secondary` | `#667085` | 次级文字 |
| `text.tertiary` | `#98A2B3` | 弱提示 |
| `accent.default` | `#5B5CE2` | 主强调色 |
| `accent.hover` | `#4D4ED0` | 强调 Hover |
| `accent.soft` | `#EEEEFF` | 轻量强调背景 |

说明：强调色采用低面积使用，避免界面形成“大面积蓝紫色后台”的观感。

---

## 5.2 Dark Theme — UI v0.1 推荐值

| Token | 推荐值 | 用途 |
|---|---:|---|
| `bg.app` | `#0D0F12` | 应用整体背景 |
| `bg.surface` | `#12151A` | 主工作区 |
| `bg.subtle` | `#171A20` | Sidebar / 次级区域 |
| `bg.hover` | `#20242B` | Hover |
| `bg.selected` | `#252748` | 选中项 |
| `border.default` | `#292E36` | 必要分割线 |
| `border.strong` | `#343A46` | 强分割线 |
| `text.primary` | `#F3F4F6` | 主文字 |
| `text.secondary` | `#A7AFBC` | 次级文字 |
| `text.tertiary` | `#788291` | 弱提示 |
| `accent.default` | `#7C7EF2` | 主强调色 |
| `accent.hover` | `#8C8EF7` | 强调 Hover |
| `accent.soft` | `#242645` | 轻量强调背景 |

Dark Theme 不使用纯黑作为大面积工作区背景，避免过强对比导致视觉疲劳。

---

# 6. 状态色 — UI v0.1 推荐值

| 状态 | 建议颜色 | 使用场景 |
|---|---|---|
| Success / Connected | `#22A06B` | 已连接、操作成功 |
| Warning | `#D99000` | 风险提示、资源偏高 |
| Danger / Failed | `#D64545` | 失败、删除、高风险 Host Key |
| Info | `#4F7DED` | 信息性反馈 |
| Neutral / Disconnected | 中性灰 | 未连接、非活跃 |

状态表达必须使用：

```text
颜色 + 图标/形状 + 文案
```

不能只依赖颜色。

例如：

```text
● Connected
○ Disconnected
◌ Connecting
! Failed
```

---

# 7. Typography — UI v0.1 推荐规范

## 7.1 UI 字体

优先使用系统字体，减少额外字体依赖和加载开销。

macOS：

```text
-apple-system / SF Pro
```

Windows：

```text
Segoe UI Variable / Segoe UI
```

跨平台 fallback：

```text
system-ui, sans-serif
```

---

## 7.2 Terminal 字体

Terminal Font 必须可配置。

建议默认：

macOS：

```text
SF Mono
```

Windows：

```text
Cascadia Mono
```

Fallback：

```text
Menlo / Consolas / monospace
```

---

## 7.3 字体层级 — UI v0.1 推荐值

| 类型 | 字号 | 字重 | 用途 |
|---|---:|---:|---|
| Page Title | 20px | 600 | Settings / Monitor 标题 |
| Section Title | 16px | 600 | 分区标题 |
| Workspace Title | 15px | 600 | 当前 Server |
| Body | 14px | 400 | 常规内容 |
| Label | 13px | 500 | 表单、工具栏 |
| Secondary | 12px | 400 | Host、说明、辅助信息 |
| Caption | 11px | 400 | 极弱辅助信息 |
| Terminal | 13–14px | 400 | 默认 Terminal 字号 |

原则：

- 不使用大量粗体
- 标题依靠字重、间距建立层级
- 表格与列表保持紧凑但不拥挤
- 中英文均需验证截断和换行

---

# 8. Spacing System — UI v0.1 推荐规范

采用 4px 基础栅格：

```text
4 / 8 / 12 / 16 / 20 / 24 / 32
```

推荐使用：

- 组件内部小间距：4–8px
- Toolbar 元素间距：8px
- 列表项内部：8–12px
- Panel Padding：12–16px
- Section 间距：20–24px
- 大页面边距：24–32px

禁止为了“留白高级感”大幅降低桌面工具的信息密度。

---

# 9. Radius / Border / Shadow — UI v0.1 推荐规范

## 9.1 Radius

```text
Small Control   6px
Button/Input     7–8px
Card/Panel       8–10px
Dialog           10–12px
```

避免过度圆润的“移动 App 卡片感”。

---

## 9.2 Border

边框只用于明确结构关系：

- Panel 分隔
- Input
- Table Header / 行分隔（必要时）
- Dialog
- Focus / Error

优先使用 1px 低对比边框。

---

## 9.3 Shadow

主界面基本不使用明显阴影。

阴影主要用于：

- Dialog
- Popover
- Context Menu
- Floating Search

阴影应轻、短、克制。

---

# 10. Iconography

图标要求：

- 统一使用线性图标体系
- 统一 Stroke 粗细
- 常规尺寸 16px
- Toolbar 可使用 18px
- 空状态主图标可使用 32–40px
- 不使用彩色拟物图标
- 不混用多套风格差异明显的图标库

图标不能替代所有文字。

高风险、低频或不熟悉操作必须带文字或 Tooltip。

---

# 11. Application Shell

## 11.1 默认结构

```text
┌───────────────────────────────────────────────────────────────┐
│ App / Global Toolbar                                         │
├──────────────┬──────────────────────────────┬─────────────────┤
│              │                              │                 │
│ Server       │ Terminal / Primary Workspace │ Quick Monitor   │
│ Sidebar      │                              │                 │
│              │                              │                 │
│              ├──────────────────────────────┴─────────────────┤
│              │ Files                                          │
├──────────────┴─────────────────────────────────────────────────┤
│ Terminal Tabs / Transfer Status                               │
└───────────────────────────────────────────────────────────────┘
```

---

## 11.2 推荐尺寸 — UI v0.1

| 区域 | 推荐尺寸 |
|---|---:|
| Global Toolbar | 44–48px 高 |
| Server Sidebar | 220–240px 宽 |
| Quick Monitor | 240–280px 宽 |
| Files | 200–260px 高，可调整 |
| Terminal Tabs | 34–38px 高 |
| Workspace Header | 44–52px 高 |

这些尺寸属于 UI 初始稿建议，应根据实际窗口大小和可用性测试微调。

---

# 12. 响应式桌面窗口行为

MauLink 是桌面应用，不需要 Web 式复杂响应式，但必须处理缩小窗口。

建议：

### 宽窗口

```text
Sidebar + Terminal + Quick Monitor + Files
```

### 中等窗口

优先折叠：

```text
Quick Monitor
```

### 较窄窗口

进一步折叠：

```text
Files
```

### 极窄窗口

P1 可启用：

```text
Server Sidebar → Narrow / Icon Mode
```

任何情况下优先保留 Terminal。

UI v0.1 建议设置最小窗口尺寸，避免核心功能压缩到不可使用状态。

---

# 13. Global Toolbar

建议包含：

左侧：

- MauLink Logo / Product Name
- 当前顶层位置（可弱化）

中间或 Sidebar 顶部：

- Search

右侧：

- `+ Server`
- Settings

原则：

- 不放置大量功能入口
- 不形成类似浏览器地址栏的复杂顶部区域
- 操作区保持稳定
- macOS 需为 Traffic Light Window Controls 留出空间

---

# 14. Server Sidebar

Server Sidebar 是 MauLink 最重要的导航区域之一。

## 14.1 结构

```text
Search

Production
  ● Web-01
    192.168.1.20
  ○ DB-01
    192.168.1.21

Development
  ● Dev-01
    dev.example.com
```

## 14.2 Server Item

每项建议包含：

- Connection State Icon
- Server Name
- Host / Description

默认不加入 CPU 小图、内存条、Sparkline 等动态监控信息。

原因：Sidebar 的任务是“找服务器、识别状态、切换”，不是 Dashboard。

## 14.3 Selected State

选中服务器采用：

- 轻量 Selected Background
- 清晰文字层级
- 可选左侧 2px Accent Indicator

不建议使用大面积高饱和背景。

## 14.4 Group Header

Group Header：

- 可折叠
- 名称清晰
- Hover 后再出现更多菜单
- 不长期显示多个操作图标

---

# 15. Welcome / Empty Home

首次启动不显示复杂 Dashboard。

页面中心或略偏上区域展示：

```text
MauLink

连接你的第一台服务器。
在一个工作区中使用 Terminal、管理文件并查看服务器状态。

[ 添加服务器 ]

了解 MauLink
```

视觉要求：

- 单一明确 Primary Action
- 不使用大面积插画
- 可使用一个简洁 Server / Terminal 线性图标
- 页面留白适中
- 用户一眼知道下一步

---

# 16. Add Server

Add Server 应采用轻量、单任务表单，不设计成多步骤 Wizard。

## 16.1 默认字段

```text
Server Name        Optional
Host               192.168.1.10
Username           root
Authentication     Password / SSH Key
Password / Key

Advanced

[ Test Connection ]   [ Connect ]
```

Port 22 可弱化放在 Host 右侧或 Advanced 中。

## 16.2 视觉层级

- Dialog / Sheet 宽度建议 480–560px
- 字段纵向排列
- Label 位于 Input 上方
- Authentication 使用 Segmented Control / Radio Group
- Advanced 使用 Disclosure Row
- Connect 为 Primary
- Test Connection 为 Secondary

## 16.3 Validation

错误直接显示在字段下方，避免仅 Toast：

```text
Host
[ 192.168.1.300 ]
请输入有效的主机地址
```

---

# 17. Edit Server

与 Add Server 保持同一表单组件和视觉结构。

主要差异：

- Title 为“编辑服务器”
- 显示已有配置
- Save Changes 为 Primary
- Delete 不与 Save 并排作为同级操作
- Delete 放在 Danger Zone 或 More 中

---

# 18. Connection Progress

点击 Connect 后必须立即提供反馈。

可以在 Workspace 内显示轻量连接状态：

```text
Connecting to Web-01...
```

必要时按阶段显示：

```text
Connecting...
Checking server identity...
Authenticating...
Opening terminal...
Connected
```

如果连接非常快，不强制播放全部阶段，避免文字闪烁。

提供：

```text
Cancel
```

避免全屏 Blocking Spinner。

---

# 19. Host Key Confirmation

首次连接属于“重要但非危险”的安全确认。

建议 Dialog：

```text
首次连接到此服务器

这是 MauLink 第一次连接这台服务器。

192.168.1.20
ED25519
SHA256:xxxxxxxxxxxxxxxx

保存该指纹后，如果服务器身份以后发生变化，
MauLink 会进行安全提醒。

什么是服务器指纹？

[ 取消 ]                   [ 信任并连接 ]
```

UI 要求：

- Fingerprint 使用 Monospace
- Server 地址清晰突出
- “信任并连接”为 Primary
- 不使用传统 SSH `yes / no` 表达

---

# 20. Host Key Changed

Host Key Changed 必须与普通 Dialog 有明显风险层级差异。

建议使用 Danger Icon + Danger Title，但不要整屏红色。

```text
服务器身份发生变化

此前保存的服务器指纹与当前服务器不一致。
这可能由服务器重装或 SSH 配置变化引起，
也可能意味着连接存在安全风险。

Previous
SHA256:xxxx

Current
SHA256:yyyy

[ 取消连接 ]

高级操作：更新已保存指纹
```

“继续连接”不能成为默认 Primary Action。

---

# 21. Server Workspace

Server Workspace 是 MauLink 的核心页面。

## 21.1 Workspace Header

建议结构：

```text
Web-01
root@192.168.1.20    ● Connected

                              Reconnect  Disconnect  ···
```

P1 可增加 Latency。

Header 不应堆积大量按钮。

## 21.2 Standard Mode

```text
Terminal + Quick Monitor + Files
```

这是日常默认模式。

## 21.3 Terminal Focus

进入后：

- Sidebar 可折叠/隐藏
- Quick Monitor 隐藏
- Files 隐藏
- Terminal 占据主要空间

适合：

- vim
- tmux
- logs
- 编译
- 长时间命令行工作

## 21.4 Monitor Focus

进入详细 Monitor 页面，Terminal 保持容易返回。

---

# 22. Terminal UI

Terminal 是 MauLink 的视觉核心区域之一。

## 22.1 Terminal Surface

原则：

- 背景纯净
- 不加卡片边框
- 不加阴影
- 不叠加装饰图形
- Text Rendering 清晰
- Cursor 清楚
- 选择区域对比明确

## 22.2 Terminal Toolbar

尽量轻量：

```text
Terminal 1                         Search(P1)   Focus   ···
```

避免占用过多垂直空间。

## 22.3 Terminal Tabs

```text
Terminal 1   Terminal 2   +
```

Tab：

- 高度 34–38px
- 当前 Tab 用底部 Accent / 背景明度区分
- Close Icon Hover 后显示或弱化显示
- `+` 一次点击创建新 Shell

## 22.4 Focus

连接完成后自动将键盘焦点放入 Terminal。

Focus Ring 不应围绕整个 Terminal 形成明显蓝框，可通过 Cursor / Header Active State 表达。

---

# 23. Files / SFTP UI

Files 默认位于 Terminal 下方。

## 23.1 Files Header

```text
←  ↑   /opt/app                                  Refresh  Upload
```

建议使用 Breadcrumb + 当前路径。

## 23.2 File Table

MVP 默认列：

```text
Name                     Size          Modified
```

Type 主要通过 Icon 表达。

表格设计：

- Compact Row
- 轻分割或无分割 + Hover Background
- 文件名优先
- Size 右对齐
- Modified 使用等宽或稳定列宽
- 文件夹始终易识别

## 23.3 Loading

刷新目录时：

- 保留路径
- 保留已有列表
- Header 显示轻量 Loading
- 新数据到达后替换

禁止 Files 整块闪白。

## 23.4 Context Menu

File：

```text
Download
Rename
Copy Path
Delete
```

Folder：

```text
Open
Download (P1)
Rename
Copy Path
Delete
```

危险操作置底并分组。

---

# 24. Transfer Center

文件传输不能使用阻塞式 Dialog。

默认底部轻量状态：

```text
Uploading 2 files · 42%                            View
```

展开：

```text
File              Progress        Speed        Status
app.tar.gz        72%             14 MB/s      Uploading
config.yml        100%            —            Completed
```

UI 要求：

- 用户传输时仍可使用 Terminal
- Progress Bar 细、稳定
- Speed 数值宽度稳定
- Failed 使用明确失败文案 + Retry
- Completed 不需要高强度绿色背景

---

# 25. Quick Monitor

Quick Monitor 的目标不是展示全部监控指标，而是让用户“不离开 Terminal 就知道服务器大致状态”。

默认只展示：

```text
CPU
Memory
Disk
Network
```

示例：

```text
CPU
24%          ▁▂▃▂▅▃▂

Memory
6.2 / 16 GB
39%

Disk
61%

Network
↓ 2.4 MB/s
↑ 380 KB/s
```

UI 要求：

- 每项占用空间小
- 当前值最突出
- Sparkline 极简
- 不使用 3D / Area Gradient Chart
- 不为每个指标使用不同高饱和颜色
- 点击指标进入 Detailed Monitor

---

# 26. Monitor Overview

Monitor Overview 第一屏只回答一个问题：

> 服务器现在健康吗？

展示：

```text
CPU
Memory
Disk
Network
Load
Uptime
```

推荐布局：

- 2–3 列简洁 Metric Blocks
- 不做传统“六张大卡片 Dashboard”
- 可以利用分区、留白、轻背景形成模块
- 当前值 > 趋势 > 辅助值

---

# 27. Monitor Detail

详细页按指标展开。

CPU：

```text
Usage
User
System
IO Wait
Core Count
Load Average
History
```

Memory：

```text
Used
Available
Cache
Buffer
Swap
History
```

Disk / Network 同理。

## 图表规范

MVP 图表：

- 2D Line Chart
- 单色或主色系
- 网格线极弱
- 支持 Hover 值
- 显示当前值与最近历史
- 不做 Zoom
- 不做 Annotation
- 不做复杂多时间范围
- 不做渐变面积填充或仅使用极弱透明填充

---

# 28. Settings

Settings 信息架构：

```text
General
Appearance
Terminal
Language
```

建议布局：

```text
Settings Sidebar | Settings Content
```

Settings 页面应保持桌面系统设置式视觉逻辑：

- 左侧分类
- 右侧设置项
- 设置项按 Section 分组
- 不把所有设置做成 Card

---

# 29. Appearance

提供：

```text
Theme
○ System
○ Light
○ Dark
```

可以使用三个小型 Theme Preview，但不得用大型视觉卡片占据大量空间。

---

# 30. Terminal Settings

至少提供：

- Font Size
- Font Family
- Scrollback
- Cursor Style

P1：Terminal Theme。

建议每项保持：

```text
Label                         Control
Description (optional)
```

避免表单过度装饰。

---

# 31. Language

支持：

```text
简体中文
English
```

切换后尽可能即时生效。

UI 必须预留英文字符串比中文更长的情况。

禁止固定按钮宽度导致英文截断。

---

# 32. Credential Prompt

当密码或 Private Key Passphrase 未保存时，使用轻量安全输入 Dialog。

建议：

```text
Web-01 需要凭据

Password
[ ••••••••••• ]

☐ Remember securely

[ Cancel ]                  [ Connect ]
```

“Remember securely”必须明确强调安全存储，不使用“Save password to config”类文案。

---

# 33. Connection Error

错误采用“两层结构”。

主层面向普通用户：

```text
无法连接 Web-01

192.168.1.20:22

无法建立 SSH 连接。

可能原因：
• 服务器未启动
• SSH 服务未运行
• 端口无法访问
• 网络连接异常

[ 重试 ]   [ 编辑连接 ]

查看详细信息
```

Details 才展示：

```text
Connection refused
ECONNREFUSED
```

错误页必须提供下一步操作，不做只有红色错误文字的死胡同。

---

# 34. Authentication Failed

建议文案：

```text
认证失败

用户名、密码或 SSH Key 可能不正确。

[ 重新输入凭据 ]   [ 编辑连接 ]

查看详细信息
```

不要仅显示：

```text
AUTH_FAILED
```

---

# 35. Delete Confirmation

删除服务器为危险操作。

Dialog：

```text
删除 Web-01？

服务器配置将从 MauLink 中移除。

☐ 同时删除已安全保存的凭据

[ 取消 ]                    [ 删除服务器 ]
```

Delete Button 使用 Danger 样式。

默认 Focus 不放在危险按钮上。

---

# 36. Loading State

原则：

- 点击后立即反馈
- 优先局部 Loading
- 避免整页 Spinner
- 不对极短任务播放完整动画
- 可取消任务显示 Cancel

推荐：

- Button Loading
- Inline Spinner
- Skeleton 仅在真正需要时使用
- 保留已有内容的 Refresh Loading

---

# 37. Empty State

所有核心页面必须有 Empty State。

Empty State 结构统一：

```text
Icon
Title
1 句解释
Primary Action
可选 Secondary Action
```

示例：

```text
还没有服务器

添加第一台服务器即可开始使用 MauLink。

[ 添加服务器 ]
```

避免插画喧宾夺主。

---

# 38. Toast / Notification

Toast 用于轻量结果反馈：

```text
连接已断开
文件上传完成
设置已保存
```

原则：

- 同时最多 2–3 条
- 自动消失的信息必须非关键
- 错误 Toast 提供可执行入口时优先带 Action
- 重要错误不要只依赖 Toast
- 不在短时间内连续弹出大量提示

---

# 39. Button Hierarchy — UI v0.1

## Primary

用于页面唯一主要动作：

```text
Connect
Add Server
Trust & Connect
Save Changes
```

## Secondary

```text
Test Connection
Retry
Edit Connection
```

## Ghost / Toolbar

```text
Refresh
Focus
More
```

## Danger

```text
Delete Server
```

一个 Dialog 通常只允许一个 Primary Action。

---

# 40. Input / Form Controls — UI v0.1

建议高度：

```text
32–36px
```

表单采用：

```text
Label
Input
Helper / Error
```

Focus：

- 明确 Focus Ring
- 不只依赖颜色细微变化

Error：

- Border + Error Text
- 不通过抖动动画提示错误

---

# 41. Context Menu / Popover

Context Menu：

- 轻阴影
- 低对比边框
- 8px 左右圆角
- Item 高度 28–32px
- 图标可选，但保持一致
- Danger Action 独立分组

Popover 打开后不要遮挡关键 Terminal 输入区域超过必要范围。

---

# 42. Interaction Motion

推荐时长：

```text
150–250ms
```

适用：

- Hover
- Expand / Collapse
- Tab Switch
- Modal
- Popover
- Status Change

禁止：

- 无限循环装饰动画
- Monitor 数据刷新动画化
- 大面积 Blur Animation
- 页面切换长过渡
- 影响输入和滚动流畅度的动画

Terminal 输入与大量输出场景下，视觉动画优先让路于性能。

---

# 43. Keyboard UX

Windows 与 macOS 使用各自平台习惯。

macOS：

```text
Command
```

Windows：

```text
Ctrl
```

至少保证：

- New Terminal
- Close Terminal
- Find（P1）
- Copy
- Paste
- Clear
- Reconnect
- Tab Navigation

Keyboard Focus 必须可见。

---

# 44. Windows / macOS 视觉策略

目标不是像素级一致，而是：

> 产品逻辑一致，操作习惯符合平台。

## macOS

尊重：

- Traffic Light Window Controls
- Command 快捷键
- System Menu
- Keychain
- macOS 字体与控件节奏

## Windows

尊重：

- Windows Window Controls
- Ctrl 快捷键
- Credential Manager
- Windows 字体与控件节奏

核心 Workspace、功能位置、数据结构保持一致。

---

# 45. Accessibility

MVP 至少满足：

- Keyboard Focus 清晰可见
- Tab Navigation 基本可用
- 状态不只依赖颜色
- 文本与背景有足够对比
- 重要图标按钮有 Accessible Name
- 图表关键数据可通过文字获取
- 中英文均不截断关键操作
- Danger / Warning 不只通过红黄颜色表达

---

# 46. Performance-related UI Requirements

UI 必须服务于产品性能目标。

已有 MVP 目标：

```text
Cold Start ≤ 1.5–2s
Idle CPU 目标 < 1%
Idle Memory 尽可能接近或低于 100MB
UI 目标 60 FPS
```

因此 UI 层明确避免：

- 大面积实时 Blur
- 大量常驻阴影
- 多个高频动画
- Server List 中实时动态图
- 隐藏页面持续绘制图表
- 不可见 Monitor 高频刷新
- Files 大量 DOM/视图节点无虚拟化策略

---

# 47. Monitor Refresh 的视觉要求

Monitor 刷新频率由业务层控制，但 UI 需要：

- 更新数值不跳动
- 图表平滑但不动画堆积
- 后台 Server 不显示持续活跃动画
- 窗口最小化时不依赖视觉刷新表达状态
- 从后台恢复时允许一次快速刷新

UI 不需要向用户暴露采样频率算法。

---

# 48. Workspace 状态记忆

MVP 建议记忆：

- Sidebar Width
- Files Height
- Quick Monitor Collapsed State
- Theme
- Language
- Last Selected Server（可选）

这些状态恢复时避免明显布局跳变。

MVP 不恢复远程 Terminal Session。

---

# 49. MVP 页面清单与 UI 交付要求

P0 必须设计：

| 编号 | 页面 / 状态 | UI 交付重点 |
|---:|---|---|
| 01 | Welcome / Empty Home | 首次使用主操作明确 |
| 02 | Server List | Group / State / Search / Selected |
| 03 | Add Server | 轻量表单 / Advanced |
| 04 | Edit Server | 与 Add Server 一致 / Danger Zone |
| 05 | Server Workspace | 三栏工作区 / Terminal First |
| 06 | Terminal | Tabs / Focus / Active State |
| 07 | Files | Path / Table / Upload / Context Menu |
| 08 | Monitor Overview | 核心指标 / 稳定数值 / 简单趋势 |
| 09 | Settings | 分类 / 表单 / Theme / Language |
| 10 | Host Key Confirmation | 安全解释 / Fingerprint |
| 11 | Connection Error | 普通用户文案 / Retry / Edit |
| 12 | Transfer Progress | 非阻塞 / Progress / Speed / Status |
| 13 | Credential Prompt | 安全输入 / Secure Remember |
| 14 | Delete Confirmation | Danger Hierarchy |

P1：

```text
Process View
Transfer Queue
Terminal Search
```

---

# 50. 未来扩展预留

MVP 不展示：

- Docker GUI
- Database GUI
- Jump Host 管理页
- Port Forward 管理页
- AI
- Plugin System

但 Server Workspace 的导航结构应允许未来增加：

```text
Overview
Terminal
Files
Monitor
Docker
Database
```

当前版本不能为了未来功能提前展示 Disabled Menu，也不能为了未来扩展牺牲 MVP 的简洁性。

---

# 51. 可直接用于 AI UI 生成工具的统一描述

以下描述可作为 Figma AI、Pen.dev 或其他 UI 生成工具的全局风格提示基线：

```text
Design a modern cross-platform desktop developer tool named MauLink.
It is an SSH + SFTP + Server Monitor workspace for Windows and macOS.

The product should feel minimal, professional, calm, precise and fast.
Avoid the visual style of a traditional admin dashboard.
Do not use large gradients, heavy glassmorphism, excessive shadows,
large colorful cards, 3D charts, decorative animations or oversized whitespace.

Use a neutral gray visual system with one low-saturation indigo accent color.
The accent color should be used sparingly for primary actions, selected states,
focus states and active tabs, never as a large background area.

The core layout is a Server Workspace:
- left: server sidebar with groups, search and connection states
- center: terminal as the dominant workspace
- right: compact quick monitor
- bottom: collapsible files/SFTP panel
- terminal tabs remain compact and immediately accessible

Terminal is always the highest-priority region.
When space becomes limited, collapse Quick Monitor first, then Files.

Use compact desktop spacing, system fonts, clear typography hierarchy,
subtle separators, minimal borders and stable layouts.
Dynamic monitor values must not cause layout shifting.

Every important state must be visible using icon/shape + text, not color alone.
Design complete loading, empty and error states.
Errors should explain the problem in normal user language and provide the next action.

The interface must support light, dark and system themes,
Simplified Chinese and English, and platform-appropriate Windows/macOS behavior.
```

---

# 52. UI 禁止项

MauLink MVP UI 禁止：

- 传统后台管理模板式设计
- 首页数据大屏
- 多彩 KPI 卡片墙
- 大面积品牌色 Sidebar
- 复杂渐变背景
- 大面积磨砂玻璃
- Neon / Cyberpunk 风格
- 3D / 拟物图表
- 每个 Panel 都有重阴影
- 每个操作都用实心按钮
- Server Sidebar 实时塞 CPU/Memory 图
- Loading 时整屏遮罩
- Error 只显示错误代码
- 状态只通过红绿颜色表达
- 过度圆角导致移动端 App 感
- 动效影响 Terminal、Scroll、Monitor 性能
- 中英文切换后按钮或标签被截断

---

# 53. UI 验收检查表

UI 设计进入实现前至少检查：

- [ ] 用户首次进入能立即找到 Add Server
- [ ] Add Server 默认表单没有暴露高级 SSH 参数
- [ ] Host Key 首次确认与 Host Key Changed 风险层级明显不同
- [ ] Server Sidebar 能一眼看出服务器名称、Host 和连接状态
- [ ] Terminal 是默认视觉焦点和最大工作区
- [ ] Quick Monitor 不压迫 Terminal
- [ ] Files 能在不离开 Workspace 的情况下使用
- [ ] Monitor 第一屏没有堆砌几十个指标
- [ ] 动态数值更新不会导致 Layout Shift
- [ ] 所有核心页面都有 Loading / Empty / Error State
- [ ] Connection Error 提供 Retry / Edit 等下一步动作
- [ ] 传输过程不阻塞 Terminal
- [ ] Light / Dark 均具备足够对比度
- [ ] 不只通过颜色表达 Connected / Failed / Warning
- [ ] Keyboard Focus 可见
- [ ] 中文和英文均不会导致主要按钮截断
- [ ] 窗口变窄时按 Terminal → Server Navigation → Quick Monitor → Files 的优先级处理
- [ ] Windows / macOS 主要快捷键符合各自平台习惯
- [ ] UI 不包含大面积 Blur、复杂动画或高成本装饰
- [ ] 整体视觉不呈现传统后台管理系统观感

---

# 54. 当前 UI 结论

MauLink 的 UI 核心不是“把 SSH 客户端做得更花哨”，而是通过更好的视觉层级和 Workspace 组织，让服务器操作变得更自然。

最终界面应始终围绕：

```text
Server
↓
Connect
↓
Workspace
├── Terminal
├── Files
└── Monitor
```

视觉上：

```text
Neutral
+ Clear Hierarchy
+ Compact Desktop Density
+ Minimal Decoration
+ Stable Dynamic Data
+ Platform-native Habits
```

交互上：

```text
少决策
少打断
少跳转
状态明确
下一步明确
```

性能上：

```text
UI 为 Terminal 和用户任务服务，
而不是让用户为 UI 效果付出性能成本。
```

这份 UI 描述文档可作为 MauLink v0.1 后续 Design System、Figma / Pen.dev 页面设计、前端组件实现和 UI 验收的统一视觉基线。
