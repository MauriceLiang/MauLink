# MauLink MVP 设计文档

**文档名称：** MauLink MVP 设计文档  
**文档版本：** v0.1  
**产品名称：** MauLink  
**阶段定位：** MVP / 第一阶段可用版本  
**目标平台：** Windows、macOS  
**界面语言：** 简体中文、English  

---

# 1. 文档目的

本文档用于从《MauLink PRD v0.1》中提炼第一阶段 MVP 的实际交付范围，明确：

- MVP 必须解决什么问题
- 第一阶段必须实现哪些功能
- 哪些功能明确不做
- 页面结构如何组织
- 核心用户流程是什么
- 性能与体验需要达到什么标准
- 何时可以认为 MVP 已完成

MVP 的目标不是构建一个功能完整的服务器管理平台，而是验证 MauLink 的核心产品价值：

> 用户是否愿意使用一个更美观、更易用、更轻量的 SSH + SFTP + Server Monitor 工具，完成日常服务器操作。

---

# 2. MVP 核心目标

MauLink MVP 必须让用户能够完整完成以下流程：

```text
启动 MauLink
↓
添加服务器
↓
连接服务器
↓
使用 Terminal
↓
浏览 / 上传 / 下载文件
↓
查看服务器性能
↓
安全退出连接
```

整个过程中要求：

- 不依赖外部 SSH 工具
- 不依赖外部 SFTP 工具
- 不依赖独立监控软件
- 用户无需阅读复杂文档
- Windows 与 macOS 都能正常使用
- 界面保持现代、简洁、易理解
- 长时间运行保持稳定
- 内存和 CPU 占用可控

---

# 3. MVP 产品定位

MVP 阶段的 MauLink 定位为：

> 一款现代、轻量、易上手的跨平台 SSH / SFTP / Server Monitor 客户端。

MVP 暂不强调：

- Docker 管理
- 数据库管理
- 企业级运维
- 大规模服务器管理
- 团队协作
- 云同步
- AI

第一阶段只验证核心价值：

```text
SSH
+
Terminal
+
SFTP
+
Monitor
+
优秀 UX
+
低资源占用
```

---

# 4. MVP 目标用户

## 4.1 开发者

需要：

- SSH 登录服务器
- 执行命令
- 上传部署文件
- 下载日志
- 查看服务器资源状态

## 4.2 学生 / 初级开发者

需要：

- 不复杂的服务器连接流程
- 图形化配置
- 易理解的错误提示
- 不要求掌握复杂 SSH 参数

## 4.3 独立开发者

需要：

- 一个工具完成 Terminal + SFTP + Monitor
- 降低工具切换成本
- 低资源占用
- Windows / macOS 双平台

---

# 5. MVP 范围

MVP 功能划分为：

```text
P0：必须完成
P1：建议完成
P2：MVP 后增强
```

---

# 6. P0 功能总览

MVP 必须包含以下模块：

```text
1. Application Shell
2. Server Manager
3. SSH Connection
4. Terminal
5. SFTP / File Manager
6. Server Monitor
7. Settings
8. Theme
9. i18n
10. Credential Security
11. Error Handling
12. Performance Control
13. Windows / macOS Packaging
```

---

# 7. Application Shell

**优先级：P0**

必须支持：

- Windows
- macOS
- 单实例应用启动
- 主窗口
- 窗口最小化 / 最大化 / 关闭
- 系统主题识别
- 基础菜单
- 应用版本信息

MVP 阶段不要求：

- Tray 常驻
- 自动更新
- 多窗口
- 插件系统

---

# 8. 主界面结构

MVP 采用三栏式 Server Workspace：

```text
┌───────────────────────────────────────────────────────────────┐
│ MauLink                                      Search   Settings │
├──────────────┬──────────────────────────────┬─────────────────┤
│              │                              │                 │
│   Servers    │          Terminal            │    Monitor      │
│              │                              │                 │
│ Production   │                              │ CPU             │
│ ● Web-01     │                              │ Memory          │
│ ● DB-01      │                              │ Network         │
│              │                              │ Disk            │
│ Development  │                              │                 │
│ ● Dev-01     ├──────────────────────────────┴─────────────────┤
│              │                   Files                         │
│              │                                                 │
├──────────────┴─────────────────────────────────────────────────┤
│ Terminal 1    Terminal 2    +                                 │
└───────────────────────────────────────────────────────────────┘
```

核心理念：

> 用户连接服务器后，Terminal、Files、Monitor 都围绕当前 Server Workspace 工作。

---

# 9. Server Manager

**优先级：P0**

必须支持：

- 新增服务器
- 编辑服务器
- 删除服务器
- 分组
- 搜索

新增字段：

- Name
- Host
- Port
- Username
- Authentication

Authentication：

- Password
- SSH Key

默认端口：

```text
22
```

基础表单只显示：

```text
Name
Host
Username
Authentication
```

Port 和更多配置放在 Advanced。

---

# 10. SSH Connection

**优先级：P0**

必须支持：

- Password Authentication
- SSH Key Authentication
- Host Key Verification
- 连接状态反馈

状态：

```text
Disconnected
Connecting
Authenticating
Connected
Failed
```

P1 可加入：

```text
Reconnecting
```

---

# 11. Host Key Verification

**优先级：P0**

第一次连接时必须展示：

```text
首次连接到此服务器

Server
192.168.1.10

Fingerprint
SHA256: ...

[ Cancel ]   [ Trust & Connect ]
```

Host Key 发生变化时必须显示高风险提示，禁止自动接受。

---

# 12. Terminal

**优先级：P0**

必须支持：

- Interactive Shell
- ANSI
- Unicode
- 中文
- IME
- Copy
- Paste
- Selection
- Scroll
- Resize
- Mouse
- Multiple Tabs

P1：

- Terminal Search

---

# 13. Multi Terminal

**优先级：P0**

同一服务器可同时打开多个 Terminal：

```text
Terminal 1
Terminal 2
Terminal 3
+
```

基本操作：

- New
- Close
- Switch

MVP 不要求：

- Split Terminal
- Drag Tab
- Workspace Restore

---

# 14. Terminal Scrollback

默认：

```text
10,000 lines
```

允许配置，但不得提供无限 Scrollback。

---

# 15. Terminal 性能要求

必须满足：

- 输入无明显延迟
- 大量输出时 UI 不冻结
- Monitor 不影响 Terminal
- SFTP 不影响 Terminal
- 长时间使用后内存不持续异常增长

大量输出需要采用有限缓冲、批量写入和背压机制。

---

# 16. SFTP / File Manager

**优先级：P0**

必须支持：

- 目录浏览
- 返回上级
- Refresh
- Upload
- Download
- Rename
- Delete
- New Directory
- Copy Path

文件信息至少展示：

- Name
- Type
- Size
- Modified Time

---

# 17. 文件传输

必须展示：

```text
File
Progress
Speed
Status
```

状态：

```text
Waiting
Transferring
Completed
Failed
Cancelled
```

MVP 必须测试：

```text
100MB
1GB
10GB
```

目标：

> 文件大小增加时，内存占用不能线性增加。

P1：

- Drag & Drop
- Transfer Queue

---

# 18. Server Monitor

**优先级：P0**

Server Monitor 是 MauLink MVP 的重要差异化功能。

目标：

> 用户不进入复杂运维平台，也能快速看懂服务器健康状态。

默认展示：

```text
CPU
Memory
Disk
Network
Load
Uptime
```

---

# 19. CPU

至少显示：

```text
Usage %
Core Count
Load
Recent Trend
```

---

# 20. Memory

至少显示：

```text
Used
Total
Usage %
Recent Trend
```

---

# 21. Disk

至少显示：

```text
Used
Total
Usage %
Primary Mount
```

---

# 22. Network

至少显示：

```text
Download
Upload
Recent Trend
```

---

# 23. System Information

至少显示：

```text
Hostname
OS
Kernel
Architecture
Uptime
```

---

# 24. Process View

**优先级：P1**

建议加入：

```text
PID
Process
CPU
Memory
User
```

第一阶段只查看，不提供 Kill Process 等危险操作。

---

# 25. Monitor 刷新策略

当前活动服务器：

```text
CPU        ~1s
Network    ~1s
Memory     ~2s
Disk       ~5s
System     Once
```

后台服务器降低刷新频率，窗口最小化时大幅降频或暂停非必要采集。

目标：

> 监控不能成为 MauLink 高 CPU 的来源。

---

# 26. Settings

**优先级：P0**

MVP 包含：

```text
General
Appearance
Terminal
Language
```

---

# 27. Appearance

支持：

```text
System
Light
Dark
```

---

# 28. Language

支持：

```text
简体中文
English
```

要求：

- 核心页面全部国际化
- 不允许核心页面硬编码单一语言
- 切换后尽可能即时生效

---

# 29. Terminal Settings

至少支持：

- Font Size
- Font Family
- Scrollback
- Cursor Style

P1：

- Terminal Theme

---

# 30. Credential Security

**优先级：P0**

敏感数据：

```text
Password
Private Key Passphrase
```

不得明文写入普通配置数据库。

推荐：

```text
macOS
→ Keychain

Windows
→ Credential Manager
```

---

# 31. Local Storage

MVP 本地数据库保存：

```text
Server Profiles
Groups
Settings
Preferences
Host Key Metadata
```

不保存明文密码。

---

# 32. Error Handling

**优先级：P0**

错误必须采用两层结构。

主层：

```text
无法连接服务器

可能原因：
• 地址错误
• SSH 服务未运行
• 端口不可访问
```

Details：

```text
Connection refused
ECONNREFUSED
```

---

# 33. Empty / Loading State

所有核心页面必须提供：

- Loading State
- Empty State
- Error State

禁止出现无解释的空白页面。

---

# 34. UI 设计目标

MVP 不是“先能用再美化”。

UI 本身属于 MVP。

设计目标：

```text
Modern
Simple
Professional
Clear
Fast
```

视觉重点：

- Typography
- Spacing
- Hierarchy
- Alignment
- Consistent Icons
- Status Feedback

---

# 35. UI 明确避免

MVP 禁止：

- 大量边框
- 复杂渐变
- 大面积毛玻璃
- 过多阴影
- Dashboard 堆叠
- 3D 图表
- 长时间持续动画
- 为美观牺牲性能

---

# 36. 动效

动效范围：

- Hover
- Expand
- Tab Switch
- Modal
- Status Change

推荐：

```text
150 ~ 250ms
```

---

# 37. 性能目标

## Cold Start

```text
≤ 1.5 ~ 2 秒
```

## Idle CPU

```text
接近 0%
目标 < 1%
```

## Memory

基础空闲窗口：

```text
尽可能接近或低于 100MB
```

不同系统 WebView 存在差异，最终以实测为准。

## UI

目标：

```text
60 FPS
```

---

# 38. 长时间运行

MVP 必须至少进行：

```text
4h+
```

持续运行测试。

Beta 前建议：

```text
8h+
```

检查：

- Memory Leak
- CPU
- SSH Stability
- Timer
- Listener
- Task
- Channel
- Socket

---

# 39. 多连接测试

MVP 最少测试：

```text
1
5
10
20
```

SSH Session。

至少保证 5~10 个日常 Session 下体验正常。

---

# 40. MVP 明确不做

第一阶段不做：

```text
Docker GUI
Database GUI
Jump Host
Port Forward
SSH Agent
RDP
VNC
FTP
Telnet
Serial
Kubernetes
Cloud Provider
Team Sync
Cloud Account
AI Agent
Plugin System
Enterprise RBAC
Remote Desktop
```

其中 Jump Host、Port Forward、SSH Agent 属于 MVP 后较早加入的高级 SSH 功能。

---

# 41. Docker 后续位置

Docker 不进入 MVP。

未来：

```text
Server Workspace
└── Docker
    ├── Containers
    ├── Images
    ├── Volumes
    ├── Networks
    ├── Logs
    └── Terminal
```

当前 MVP 架构不能阻碍未来增加 Docker，但不能为了未来 Docker 过度设计。

---

# 42. Database 后续位置

Database 不进入 MVP。

未来：

```text
Server Workspace
└── Database
    ├── MySQL
    ├── PostgreSQL
    ├── Redis
    └── SQLite
```

MVP 只需保证未来可通过 SSH Tunnel 扩展。

---

# 43. MVP 页面清单

必须设计和实现：

```text
01. Welcome / Empty Home
02. Server List
03. Add Server
04. Edit Server
05. Server Workspace
06. Terminal
07. Files
08. Monitor Overview
09. Settings
10. Host Key Confirmation
11. Connection Error
12. Transfer Progress
13. Credential Prompt
14. Delete Confirmation
```

P1：

```text
15. Process View
16. Transfer Queue
17. Terminal Search
```

---

# 44. 核心流程

## 首次使用

```text
Launch
↓
No Server Empty State
↓
Add Server
↓
Input Connection Info
↓
Connect
↓
Host Key Confirmation
↓
Authentication
↓
Server Workspace
```

## 日常连接

```text
Launch
↓
Server List
↓
Click Server
↓
Connect
↓
Workspace
↓
Terminal
```

## 上传文件

```text
Connected Server
↓
Files
↓
Upload
↓
Select File
↓
Transfer
↓
Progress
↓
Complete
```

## 查看服务器状态

```text
Connected Server
↓
Overview
↓
CPU / Memory / Disk / Network
↓
Open Detailed Monitor
```

---

# 45. MVP 数据边界

必须限制：

```text
Terminal Scrollback
Monitor Samples
Transfer History
Application Logs
Event Queue
```

禁止任何集合无边界增长。

---

# 46. Background Work 原则

所有后台任务必须做到：

```text
Visible
→ Normal Frequency

Background
→ Reduced Frequency

Minimized
→ Minimum Frequency / Pause

Disconnected
→ Stop
```

---

# 47. MVP 验收标准

MVP 只有同时满足以下四类要求才算完成。

## 功能验收

必须通过：

- Server CRUD
- Group
- Search
- Password SSH
- SSH Key
- Host Key
- Terminal
- Multi Terminal
- SFTP Browse
- Upload
- Download
- Rename
- Delete
- Monitor
- Settings
- Theme
- Language
- Credential Storage

## UX 验收

必须保证：

- 首次连接路径清晰
- 关键状态均有反馈
- 错误信息普通用户能理解
- 无核心页面空白状态
- 界面视觉统一
- Windows/macOS 操作习惯基本正确
- 中英文布局不出现明显溢出

## 性能验收

必须测试：

```text
Cold Start
Idle CPU
Idle Memory
Terminal Stress
SFTP Large File
Multi Session
Monitor Overhead
Long Running
```

不得存在明显：

- 内存泄漏
- 空闲高 CPU
- Terminal 卡顿
- 大文件导致内存暴涨
- Monitor 导致界面持续掉帧

## 安全验收

必须确认：

- Password 不明文保存
- Passphrase 不明文保存
- Host Key 正确校验
- Host Key Changed 有明显警告
- Log 不泄露敏感信息
- 删除服务器可删除关联 Credential

---

# 48. MVP Exit Criteria

只有满足以下条件，MVP 才允许进入 Beta：

```text
核心 P0 功能全部完成
+
核心流程无阻塞问题
+
Windows 可正常使用
+
macOS 可正常使用
+
无已知严重安全问题
+
无严重内存泄漏
+
Terminal 长时间稳定
+
SFTP 大文件稳定
+
Monitor 数据正确且性能可接受
+
UI 达到可公开展示水平
```

---

# 49. MVP 后第一批增强

MVP 稳定后优先加入：

```text
1. Drag & Drop Upload
2. Auto Reconnect
3. Terminal Search
4. Process View
5. Transfer Queue
6. Command Palette
7. Advanced SSH
```

之后再进入：

```text
Docker
Database
```

---

# 50. MVP 开发顺序建议

```text
Phase 1
Application Shell
Theme
i18n
UI Foundation

↓

Phase 2
Server Manager
Credential
Local Storage

↓

Phase 3
SSH Connection
Host Key
Terminal

↓

Phase 4
Multi Terminal
Terminal UX
Performance

↓

Phase 5
SFTP
File Manager
Transfer

↓

Phase 6
Server Monitor
Charts
Adaptive Sampling

↓

Phase 7
Settings
Error State
Empty State
UX Polish

↓

Phase 8
Performance Test
Security Test
Cross-platform Test

↓

MVP Beta
```

---

# 51. MVP 成功定义

MauLink MVP 成功，不以代码量或功能数量判断。

真正的成功标准是：

> 用户第一次打开 MauLink 后，可以自然地添加服务器、连接、使用 Terminal、上传文件和查看服务器状态，并明显感觉到 MauLink 比传统 SSH 工具更加美观、简单、流畅，同时不会因为长时间运行而产生明显的性能负担。

MVP 的核心价值：

```text
Beautiful
+
Simple
+
Fast
+
Stable
+
Useful
```

---

# 52. 当前结论

MauLink MVP 必须严格聚焦：

```text
Server Manager
+
SSH
+
Terminal
+
SFTP
+
Server Monitor
+
Settings
+
Security
+
UX
+
Performance
```

第一阶段不做 Docker、不做数据库、不做复杂高级 SSH。

MVP 的任务不是证明 MauLink“什么都能做”。

而是先证明：

> **MauLink 能把最常用的服务器工作流做得足够漂亮、简单、流畅和稳定。**
