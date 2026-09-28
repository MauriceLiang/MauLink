# MauLink UX 设计文档

**文档名称：** MauLink UX 设计文档  
**文档版本：** v0.1  
**产品名称：** MauLink  
**对应需求基线：** MauLink PRD v0.1 / MauLink MVP 设计文档 v0.1  
**设计阶段：** UX / 信息架构 / 用户流程 / 交互设计  
**目标平台：** Windows、macOS  
**界面语言：** 简体中文、English  

---

# 1. 文档目的

本文档用于定义 MauLink MVP 的用户体验架构，包括：

- 信息架构
- 页面关系
- 核心用户流程
- 主导航逻辑
- Server Workspace 交互模型
- SSH 连接体验
- Terminal 交互
- Files / SFTP 交互
- Server Monitor 交互
- 状态反馈
- 错误处理
- 新手引导
- 快捷操作
- Windows / macOS 体验差异
- 可访问性与易用性要求

本文档不负责确定最终视觉风格，例如：

- 品牌色
- 精确字号
- 圆角数值
- 阴影参数
- 图表配色
- Logo

这些内容在后续 UI Design System 阶段确定。

---

# 2. UX 核心目标

MauLink 的核心体验目标：

> 用户第一次使用 MauLink，也能在没有阅读帮助文档的情况下完成服务器连接、Terminal 操作、文件传输和性能查看。

体验优先级：

```text
清晰
↓
易学
↓
高效
↓
可预期
↓
流畅
↓
高级能力
```

不能为了高级用户效率牺牲新用户理解成本。

---

# 3. UX 设计原则

## 3.1 一眼知道下一步做什么

任何页面都不能让用户停下来思考：

> “我现在应该点哪里？”

空页面必须有明确主操作。

例如：

```text
还没有服务器

添加第一台服务器即可开始使用 MauLink。

[ 添加服务器 ]
```

---

## 3.2 高频操作一步可达

高频功能：

- 连接服务器
- 新建 Terminal
- 上传文件
- 查看 Monitor
- 搜索服务器
- 切换服务器

应尽量在 1~2 次操作内完成。

---

## 3.3 高级能力后置

默认界面不展示：

- Jump Host
- Proxy
- Algorithm
- Compression
- Environment
- Keep Alive 细节
- SSH Cipher

高级能力通过：

```text
Advanced
```

进入。

---

## 3.4 不暴露不必要的技术复杂度

底层错误：

```text
ECONNREFUSED
AUTH_FAILED
HOST_KEY_MISMATCH
```

不能直接作为主错误文案。

主层必须转换为用户语言。

---

## 3.5 所有状态可感知

用户必须能够知道当前处于：

```text
未连接
连接中
认证中
已连接
正在重连
连接失败
正在传输
传输完成
```

禁止无状态反馈。

---

## 3.6 不因为美观降低效率

视觉效果不能影响：

- Terminal 输入
- 页面切换
- 滚动
- Monitor 刷新
- 文件列表操作

---

# 4. 核心体验模型

MauLink 不采用传统：

```text
服务器页面
Terminal 页面
SFTP 页面
Monitor 页面
```

完全分离的产品结构。

核心模型是：

> Server Workspace

即每台服务器都有自己的工作空间。

---

# 5. 顶层信息架构

```text
MauLink
│
├── Servers
│   ├── Server Groups
│   ├── Server Search
│   ├── Add Server
│   └── Server Context Menu
│
├── Workspace
│   ├── Overview
│   ├── Terminal
│   ├── Files
│   └── Monitor
│
└── Settings
    ├── General
    ├── Appearance
    ├── Terminal
    └── Language
```

未来扩展：

```text
Workspace
├── Docker
└── Database
```

---

# 6. 主应用导航

主窗口建议采用：

```text
左侧：Server Sidebar
中间：Primary Workspace
右侧：Context Panel / Quick Monitor
底部或顶部：Terminal Tabs / Workspace Tabs
```

原则：

- Server 永远容易找到
- Terminal 永远是 Workspace 核心
- Monitor 不应完全抢占 Terminal 空间
- Files 能与 Terminal 协同使用

---

# 7. 主界面默认布局

推荐：

```text
┌────────────────────────────────────────────────────────────────────┐
│ MauLink        Search                           + Server   Settings │
├──────────────┬────────────────────────────────────┬────────────────┤
│              │                                    │                │
│ SERVERS      │          TERMINAL                  │ QUICK MONITOR  │
│              │                                    │                │
│ Production   │  $                                 │ CPU     24%    │
│ ● Web-01     │                                    │ Memory  42%    │
│ ● Web-02     │                                    │ Disk    61%    │
│              │                                    │ Network ↓ ↑    │
│ Development  │                                    │                │
│ ● Dev-01     │                                    │                │
│              ├────────────────────────────────────┴────────────────┤
│              │ FILES                                               │
│              │ /opt/app                                            │
│              │                                                      │
├──────────────┴─────────────────────────────────────────────────────┤
│ Terminal 1        Terminal 2        +                              │
└────────────────────────────────────────────────────────────────────┘
```

注意：

这只是 UX 布局关系，不代表最终视觉稿。

---

# 8. Workspace 布局模式

为了兼顾不同用户，Workspace 建议提供 3 种布局状态。

## 8.1 Standard

默认：

```text
Terminal + Quick Monitor + Files
```

适合日常开发。

---

## 8.2 Terminal Focus

Terminal 最大化：

```text
Terminal
```

Quick Monitor 折叠。

Files 折叠。

适合：

- vim
- tmux
- logs
- 编译
- 长时间 Terminal 工作

快捷入口：

```text
Focus Terminal
```

---

## 8.3 Monitor Focus

用于详细监控：

```text
CPU
Memory
Network
Disk
System
Process
```

Terminal 保持可快速返回。

---

# 9. Server Sidebar

Server Sidebar 是主要导航之一。

每个 Server Item 建议显示：

```text
● Web-01
  192.168.1.20
```

连接状态：

```text
● Connected
○ Disconnected
◌ Connecting
! Failed
```

不能只依赖颜色。

图标、状态或辅助文字至少保留一种额外表达方式。

---

# 10. Server Group

支持：

```text
Production
Development
Personal
```

Group 默认可折叠。

折叠状态需要记忆。

Group Header 操作：

```text
Expand / Collapse
Context Menu
```

避免长期显示大量操作按钮。

---

# 11. Server 搜索

搜索入口位于 Server Sidebar 顶部或全局顶部。

输入后实时过滤：

- Name
- Host
- Group

搜索结果无需切换页面。

ESC：

```text
Clear Search
```

---

# 12. Server Context Menu

右键服务器：

```text
Connect
Open New Terminal
Edit
Duplicate（P1）
Move to Group
Delete
```

Connected 状态下：

```text
Disconnect
Open New Terminal
Files
Monitor
Edit
```

---

# 13. 首次启动体验

首次启动不得展示复杂 Dashboard。

推荐：

```text
MauLink

连接你的第一台服务器。

在一个工作区中使用 Terminal、管理文件并查看服务器状态。

[ 添加服务器 ]
```

次要操作：

```text
了解 MauLink
```

MVP 可不提供复杂 onboarding carousel。

---

# 14. Add Server 流程

点击：

```text
+ Add Server
```

进入轻量表单。

默认字段：

```text
Server Name       可选 / 可自动生成

Host
192.168.1.10

Username
root

Authentication
● Password
○ SSH Key

Password
••••••••

[ Test Connection ]     [ Connect ]
```

---

# 15. Add Server 表单原则

默认隐藏：

```text
Port 22
Keep Alive
Timeout
Proxy
Algorithm
Startup Command
```

通过：

```text
Advanced
```

展开。

Port 可以在 Host 右侧以弱化方式显示。

---

# 16. Server Name

如果用户没有输入名称：

可自动使用：

```text
hostname
```

或：

```text
username@host
```

避免强迫用户填写非必要字段。

---

# 17. Test Connection

Test Connection 不应该直接进入 Workspace。

流程：

```text
点击 Test Connection
↓
检查网络
↓
SSH Handshake
↓
Authentication
↓
显示结果
```

成功：

```text
✓ Connection successful
```

失败：

显示可理解原因。

---

# 18. Connect 行为

用户点击 Connect：

```text
Validate
↓
Connecting
↓
Host Key
↓
Authenticating
↓
Connected
↓
Workspace
```

任何阶段均要有状态。

---

# 19. Host Key UX

首次 Host Key：

```text
首次连接到此服务器

这是 MauLink 第一次连接这台服务器。

192.168.1.20

ED25519
SHA256:xxxxxxxxxxxx

保存该指纹后，如果服务器身份以后发生变化，
MauLink 会进行安全提醒。

[ 取消 ]          [ 信任并连接 ]
```

提供：

```text
什么是服务器指纹？
```

作为辅助说明。

---

# 20. Host Key Changed

这属于高风险状态。

不能沿用普通 Dialog。

推荐：

```text
服务器身份发生变化

此前保存的服务器指纹与当前服务器不一致。

这可能是服务器重装、SSH 配置发生变化，
也可能意味着连接存在安全风险。

Previous
SHA256:xxxx

Current
SHA256:yyyy

[ 取消连接 ]

高级操作：
更新已保存指纹
```

“继续连接”不能作为默认 Primary Action。

---

# 21. Connection Loading

连接时 Workspace 可打开，但必须显示：

```text
Connecting to Web-01...
```

避免整屏 blocking spinner。

用户可：

```text
Cancel
```

---

# 22. Connection Error

错误页或 Dialog：

```text
无法连接 Web-01

192.168.1.20:22

无法建立 SSH 连接。

可能原因：
• 服务器未启动
• SSH 服务未运行
• 端口无法访问
• 网络连接异常

[ 重试 ] [ 编辑连接 ]

查看详细信息
```

---

# 23. Workspace Header

连接后 Workspace Header 建议包含：

```text
Web-01
root@192.168.1.20

● Connected

Latency 24ms（P1）
```

主要操作：

```text
Reconnect
Disconnect
More
```

不要放过多按钮。

---

# 24. Terminal 交互

Terminal 是 Workspace 默认焦点。

连接完成后：

> 自动将键盘焦点放入 Terminal。

用户无需再次点击。

---

# 25. Terminal Tabs

建议使用：

```text
Terminal 1
Terminal 2
+
```

点击 `+`：

直接创建新 Shell。

关闭 Tab：

- 正常无运行任务：直接关闭
- 仍有明显前台任务：MVP 可统一提示是否关闭 Session

---

# 26. Terminal Tab 状态

Tab 可显示：

```text
Terminal 1
●
```

或其他轻量状态。

不要持续显示大量动态信息。

---

# 27. Terminal Copy / Paste

macOS：

```text
Cmd + C
Cmd + V
```

Windows：

需要避免 `Ctrl + C` 与 SIGINT 冲突。

建议遵循 Terminal 常见逻辑：

- 有选中文本时 Ctrl/Cmd + C → Copy
- 无选中内容时 Ctrl + C → 发送 SIGINT（Windows 需结合 Terminal 习惯验证）

最终行为需要在可用性测试阶段确认。

---

# 28. Terminal Search

P1。

快捷键：

```text
Cmd/Ctrl + F
```

搜索栏浮于 Terminal 顶部。

ESC 关闭。

---

# 29. Terminal Focus Mode

用户可通过：

- 快捷键
- Terminal Header
- 双击区域

进入 Focus。

Focus Mode：

```text
隐藏 / 折叠 Sidebar
隐藏 Quick Monitor
隐藏 Files
```

再次操作恢复。

---

# 30. Files 区域

Files 属于当前 Server Workspace。

默认位置：

```text
Terminal 下方
```

可调整高度。

建议允许：

```text
Collapse
Expand
Open Full Files View
```

---

# 31. Files 默认信息

显示：

```text
Name
Size
Modified
```

Type 可通过 Icon 表达。

高级：

```text
Permission
Owner
```

P1 或详细模式显示。

---

# 32. Directory Navigation

Files Header：

```text
←  ↑   /opt/app                      Refresh
```

支持：

- Back
- Parent
- Path Breadcrumb
- 输入 Path（P1）
- Refresh

---

# 33. File 双击

Folder：

```text
进入目录
```

File：

MVP 不默认提供文本编辑器。

建议：

```text
Download
Open with system（P1）
```

避免第一阶段扩展成 IDE。

---

# 34. File Context Menu

```text
Download
Rename
Copy Path
Delete
```

Folder：

```text
Open
Download（P1）
Rename
Copy Path
Delete
```

---

# 35. Upload

主入口：

```text
Upload
```

P1 支持：

```text
Drag & Drop
```

上传开始后，不使用阻塞 Dialog。

进入：

```text
Transfer Center
```

---

# 36. Transfer Center

建议使用底部轻量状态栏：

```text
Uploading 2 files · 42%

[ View ]
```

展开后：

```text
File              Progress    Speed       Status
app.tar.gz        72%         14 MB/s     Uploading
config.yml        100%        —           Completed
```

用户可以继续使用 Terminal。

---

# 37. Transfer Failure

失败后：

```text
config.yml
Upload failed

Permission denied

[ Retry ]
```

不要只显示：

```text
Failure
```

---

# 38. Quick Monitor

Quick Monitor 解决：

> 用户不离开 Terminal 就能知道服务器大致健康状况。

默认：

```text
CPU
Memory
Disk
Network
```

每个指标只显示核心数据。

---

# 39. Quick Monitor 信息层级

示例：

```text
CPU
24%
▁▂▃▂▅▃▂

Memory
6.2 / 16 GB
39%

Disk
61%

Network
↓ 2.4 MB/s
↑ 380 KB/s
```

避免塞入：

```text
user
system
iowait
cache
buffer
swap
```

这些进入详细 Monitor。

---

# 40. Quick Monitor 点击

点击 CPU：

进入：

```text
Detailed Monitor → CPU
```

点击 Memory：

进入：

```text
Detailed Monitor → Memory
```

---

# 41. Monitor Focus 页面

结构：

```text
Monitor

Overview
CPU
Memory
Disk
Network
System
Processes（P1）
```

可以使用：

- 顶部 Tabs
或
- 二级 Sidebar

优先选择不会与 Server Sidebar 产生混淆的方案。

---

# 42. Monitor Overview

第一屏回答：

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

不要让用户第一屏看到几十个数字。

---

# 43. Monitor Detail

Detail 页面才展示：

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

---

# 44. Monitor 图表交互

MVP 图表应尽量简单。

支持：

- Hover 查看具体值
- 当前值
- 最近历史

MVP 不要求：

- Zoom
- 多时间范围
- Export
- Annotation

---

# 45. Monitor 后台行为

用户离开当前 Server 后：

Quick Monitor 不再持续高频更新。

UX 上不需要向用户暴露复杂采样机制。

只需确保：

- 返回时迅速恢复
- 不出现明显陈旧状态误导

---

# 46. Disconnected Workspace

如果 Server 断开：

Terminal 保留已有内容。

显示：

```text
Connection lost

[ Reconnect ]
```

Files 和 Monitor 进入：

```text
Disconnected
```

但不要清空已显示的数据。

可显示：

```text
Last updated 20s ago
```

---

# 47. Reconnect

P1 自动重连。

手动重连：

```text
Reconnect
```

成功后：

- 新 Terminal Session 可自动建立
- 原 Terminal 是否恢复取决于 SSH Session 能力，MVP 不承诺恢复远端 Shell 状态

避免给用户产生“原进程一定恢复”的错误预期。

---

# 48. Settings 信息架构

```text
Settings
├── General
├── Appearance
├── Terminal
└── Language
```

采用普通用户容易理解的分类。

---

# 49. General

MVP 可包含：

```text
Launch Behavior（P1）
Confirm Before Disconnect
Default Download Folder
```

不要在 General 堆 SSH 高级配置。

---

# 50. Appearance

```text
Theme
● System
○ Light
○ Dark
```

UI 预览可后续增加。

---

# 51. Terminal Settings

包含：

```text
Font
Font Size
Cursor
Scrollback
```

每项尽量带即时预览。

---

# 52. Language

```text
Language

● 简体中文
○ English
```

切换后立即更新界面。

---

# 53. Delete Server

危险操作。

流程：

```text
Delete Web-01?

This will remove the server profile from MauLink.

☑ Also remove saved credentials

[ Cancel ] [ Delete ]
```

Delete 按钮使用危险操作视觉层级。

---

# 54. Disconnect

普通 Disconnect 不需要复杂确认。

但如果：

- 正在上传文件
- 正在下载文件

需要提示：

```text
2 transfers are still active.

Disconnecting will stop them.

[ Cancel ] [ Disconnect ]
```

---

# 55. Keyboard UX

MVP 应优先支持：

```text
Cmd/Ctrl + K        Search / Quick Open（P1）
Cmd/Ctrl + T        New Terminal
Cmd/Ctrl + W        Close Current Terminal
Cmd/Ctrl + F        Terminal Search（P1）
Cmd/Ctrl + ,        Settings
```

实际快捷键需要避免与 Terminal shell 行为冲突。

---

# 56. 快速连接

P1。

可通过：

```text
Search / Quick Open
```

输入：

```text
web
```

快速找到服务器并连接。

MVP 初期可仅通过 Server Search 实现。

---

# 57. Loading 设计原则

禁止：

```text
整页 Spinner
```

除非应用首次初始化。

优先 Skeleton / Inline Loading。

例如 Monitor：

```text
CPU
Loading...
```

而 Terminal 仍可使用。

---

# 58. Empty State 清单

必须设计：

- No Server
- Empty Group
- No Search Result
- Empty Directory
- No Transfer
- No Monitor Data
- No Process Result（P1）

每个 Empty State 都应该回答：

```text
发生了什么？
为什么？
用户下一步能做什么？
```

---

# 59. Error State 清单

至少覆盖：

```text
Connection Refused
Authentication Failed
Host Key Changed
Network Timeout
SFTP Permission Denied
File Not Found
Transfer Failed
Monitor Unsupported
Monitor Command Failed
Credential Access Failed
```

---

# 60. Authentication Failed

示例：

```text
登录失败

服务器拒绝了当前用户名或密码。

root@192.168.1.20

[ 修改凭据 ] [ 重试 ]

详细信息
```

---

# 61. Monitor Unsupported

某些服务器可能不支持预期 Linux 命令。

不能让整个 Monitor 崩溃。

示例：

```text
部分监控信息不可用

MauLink 无法读取 Disk I/O 数据，
其他指标仍可正常使用。

[ 查看原因 ]
```

采用局部降级。

---

# 62. 新手帮助策略

MVP 不使用强制教程。

使用：

- Contextual Help
- Tooltip
- Empty State
- Learn More
- Progressive Disclosure

例如：

```text
SSH Key ?
```

Hover / Click：

简要解释。

---

# 63. Tooltip 原则

Tooltip 用于解释：

- 不常见图标
- 技术术语
- 快捷键

不能将关键业务说明全部藏在 Tooltip 中。

---

# 64. 用户反馈原则

操作成功不必全部弹 Toast。

例如：

```text
Tab 切换
文件夹进入
Server 切换
```

无需提示。

适合 Toast：

```text
Server saved
Upload completed
Copied path
Settings updated
```

错误使用明显 Error Message。

---

# 65. Toast 数量

避免连续任务产生几十个 Toast。

多个文件上传：

错误：

```text
file1 uploaded
file2 uploaded
file3 uploaded
...
```

正确：

```text
12 files uploaded successfully
```

---

# 66. 危险操作原则

以下操作必须明确确认：

- Delete Server
- Delete Remote File
- Delete Directory
- Clear Saved Credential
- Future Docker Delete
- Future Database destructive action

---

# 67. 跨平台 UX

Windows 和 macOS 功能一致，但尊重平台习惯。

macOS：

```text
Command
System menu
Traffic light window controls
Keychain
```

Windows：

```text
Ctrl
Windows window controls
Credential Manager
```

不强行让两个平台像素级一致。

目标：

> 产品逻辑一致，操作习惯符合平台。

---

# 68. 响应式窗口行为

MauLink 是桌面应用，但窗口可能缩小。

建议定义最小宽度。

窗口变窄：

```text
Quick Monitor
→ Collapse

Files
→ Collapse

Server Sidebar
→ Narrow / Icon Mode（P1）
```

Terminal 必须优先保留。

---

# 69. 默认优先级

窗口空间不足时：

```text
Terminal
>
Server Navigation
>
Quick Monitor
>
Files
```

Terminal 永远是最高优先显示区域。

---

# 70. 可访问性

至少保证：

- Keyboard Focus 可见
- 不只通过颜色表示状态
- 文本与背景有足够对比
- 重要按钮有明确文本或 Accessible Name
- Tab Navigation 基本可用
- 中英文不会因长度差异导致按钮截断

---

# 71. UX 性能感知

除了真实性能，还需要降低用户的“等待感”。

原则：

- 点击立即反馈
- 连接阶段显示明确进度
- 页面局部更新
- 避免长时间无变化
- 可取消的任务提供 Cancel

---

# 72. Connection Progress

连接过程可显示：

```text
Connecting...
Checking server identity...
Authenticating...
Opening terminal...
Connected
```

如果过程非常快，不强制逐条播放，避免闪烁。

---

# 73. 视觉稳定性

Monitor 数据刷新时：

禁止布局不断跳动。

例如：

```text
9%
→
100%
```

宽度变化不能导致其他元素移动。

数值区域预留稳定宽度。

---

# 74. Files 视觉稳定性

加载目录时保留：

- 当前路径
- 已有列表

刷新期间使用轻量 Loading。

避免整个 Files 面板闪白。

---

# 75. Terminal 与 Files 联动

P1 可加入：

Terminal 当前目录 → Files：

```text
Open Current Directory in Files
```

反向：

Files 某目录：

```text
Open Terminal Here
```

这是未来非常有价值的 Workspace 协同能力。

MVP 可以预留交互位置，但不强制实现。

---

# 76. Workspace 状态记忆

MVP 建议记忆：

- Sidebar width
- Files height
- Quick Monitor collapsed state
- Last selected Server（可选）
- Theme
- Language

MVP 不要求恢复远程 Terminal Session。

---

# 77. UX 验收任务

必须邀请测试用户完成以下任务：

## Task 1

```text
添加一台服务器并连接
```

观察：

- 是否知道哪里添加
- 是否理解 Authentication
- 是否理解 Host Key

---

## Task 2

```text
创建第二个 Terminal
```

观察：

- 是否找到 +
- 是否理解 Tab

---

## Task 3

```text
上传一个文件到 /opt/app
```

观察：

- 是否能找到 Files
- 是否能理解 Transfer Progress

---

## Task 4

```text
查看服务器当前内存使用情况
```

观察：

- 是否能快速找到 Memory
- 是否需要进入复杂页面

---

## Task 5

```text
连接失败后修改服务器端口并重新连接
```

观察：

- Error 是否提供下一步动作
- Edit 是否容易找到

---

# 78. UX 成功指标

MVP UX 达标的核心判断：

- 用户无需说明即可找到 Add Server
- 用户能够独立完成 SSH 连接
- 用户能够理解 Host Key 提示的大致意义
- 用户能够快速打开第二个 Terminal
- 用户能够完成 Upload / Download
- 用户能够在数秒内找到 CPU / Memory
- 出错时用户知道下一步做什么
- 页面布局不会让 Terminal 显得拥挤
- 长时间操作不存在大量无意义弹窗

---

# 79. UX 阶段输出物

本阶段后续需要继续产出：

```text
1. Information Architecture
2. User Flow
3. Low-Fidelity Wireframe
4. Interaction Specification
5. State Matrix
6. UI Design System
7. High-Fidelity UI
8. Prototype
9. Usability Test
```

本文档已经完成前四项中的基础定义。

下一阶段应优先绘制：

- Welcome
- Add Server
- Main Workspace
- Terminal Focus
- Files
- Monitor Overview
- Settings
- Host Key Dialog
- Error State

低保真 Wireframe。

---

# 80. 当前 UX 结论

MauLink UX 的核心不是增加更多操作入口，而是减少用户决策。

核心流程：

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

必须始终保持简单。

MVP UX 最终应该让用户产生的感受是：

> “我不需要学习 MauLink，它打开以后我自然就知道怎么用。”

这将作为 MauLink MVP 后续 UI 设计和交互实现的 UX 基线。
