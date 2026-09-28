# MauLink 产品需求文档（PRD）

**文档名称：** MauLink 产品需求文档  
**文档版本：** v0.1  
**项目阶段：** 立项 / 产品定义  
**产品名称：** MauLink  
**产品类型：** 跨平台 SSH / SFTP / Server Workspace  
**目标平台：** Windows、macOS  
**界面语言：** 简体中文、English  
**产品定位：** 免费、开源、现代、轻量、高性能的开发者服务器工作台  

---

# 1. 文档目的

本文档用于明确 MauLink 的产品目标、目标用户、核心场景、功能范围、体验要求、非功能性要求、版本边界和验收标准，作为后续 UX/UI 设计、系统架构设计、技术设计、开发计划、测试和发布的需求基线。

本文档重点回答：

- 为什么做 MauLink
- MauLink 为谁服务
- 用户通过 MauLink 解决什么问题
- 第一阶段必须做什么
- 哪些功能暂时不做
- 用户体验必须达到什么水平
- 性能和资源占用需要达到什么标准
- 如何判断一个版本是否满足发布条件

本文档不负责描述具体代码结构、类设计、数据库表结构、Rust 模块拆分等技术实现细节，这些内容应在后续技术设计文档中定义。

---

# 2. 产品背景

目前常见 SSH 客户端虽然能够满足基本远程连接需求，但在实际使用中仍存在以下问题：

1. 部分核心功能需要付费。
2. 免费版本经常存在功能限制。
3. 部分产品界面设计偏传统，学习成本较高。
4. Terminal、SFTP、服务器监控等功能相互割裂。
5. 一些跨平台客户端启动较慢、内存占用较高。
6. 新用户面对 SSH Host Key、私钥、端口转发、Jump Host 等概念时学习成本较高。
7. 服务器监控功能通常过于简单，或者过于偏向专业运维平台。
8. Docker、数据库、日志、文件管理等服务器相关工作需要在多个工具之间频繁切换。
9. Windows 与 macOS 上的使用体验往往存在明显差异。
10. 很多工具以功能数量为导向，而非以整体体验为导向。

MauLink 希望解决的不是“SSH 是否能连接”，而是：

> 如何让开发者以更简单、更美观、更高效、更低资源占用的方式管理自己的服务器。

---

# 3. 产品愿景

MauLink 的长期愿景是：

> 成为开发者每天都会打开的轻量服务器工作台。

用户不需要在多个工具之间频繁切换，而是在一个统一的工作区内完成：

```text
连接服务器
→ 使用 Terminal
→ 管理文件
→ 查看服务器状态
→ 管理 Docker
→ 连接数据库
→ 排查问题
```

最终产品形态：

```text
MauLink
└── Server Workspace
    ├── Overview
    ├── Terminal
    ├── Files
    ├── Monitor
    ├── Docker
    └── Database
```

---

# 4. 产品定位

MauLink 定位为：

> 一款面向开发者和轻量运维场景的免费开源跨平台 SSH / SFTP 服务器管理客户端。

核心关键词：

- 免费
- 开源
- 美观
- 易用
- 轻量
- 高性能
- 跨平台
- 双语
- 一体化
- 开发者友好

MauLink 不追求第一阶段成为功能最多的服务器管理软件，而是优先把最常用的核心体验做到优秀。

---

# 5. 产品原则

## 5.1 UX First

用户体验优先于功能数量。

所有功能在进入开发前必须回答：

- 用户是否真的需要
- 是否降低用户完成任务的成本
- 是否让界面更复杂
- 是否会给新手增加额外学习负担
- 是否能够通过更简单的交互实现

---

## 5.2 Performance First

性能属于产品需求，而不是后期优化项。

MauLink 必须长期关注：

- 启动速度
- 内存占用
- 空闲 CPU
- UI 流畅度
- Terminal 输入延迟
- 大量输出性能
- SFTP 大文件传输
- 多连接性能
- 后台任务数量
- 长时间运行稳定性

---

## 5.3 Progressive Disclosure

采用渐进式信息展示。

新用户默认只看到完成当前任务所需的最少信息。

高级选项应隐藏在 Advanced / More Settings 中。

例如添加服务器时默认展示：

```text
Host
Username
Authentication
Password / SSH Key
Connect
```

而以下内容放入高级设置：

```text
Port
Jump Host
Proxy
Timeout
Keep Alive
Compression
Algorithm
Startup Command
Environment
```

目标：

> 新手第一次打开就会用，高级用户仍然能够完成复杂配置。

---

## 5.4 Cross-platform First

Windows 与 macOS 均属于一等平台。

不得出现：

- 仅 Windows 功能完整
- macOS 长期缺失功能
- 快捷键只适配单一平台
- UI 明显偏向单一操作系统

允许针对不同平台使用不同系统能力，但必须保证核心体验一致。

---

## 5.5 Free & Open Source

MauLink 核心功能长期保持：

- 免费
- 开源
- 无付费墙
- 无基础功能会员限制

如果未来产生发行、签名、云同步等外部成本，应优先通过可选方式解决，不影响本地核心功能免费使用。

---

# 6. 目标用户

## 6.1 核心用户

### 开发者

特点：

- 经常 SSH 登录 Linux 服务器
- 使用 Docker
- 需要上传、下载、编辑服务器文件
- 需要查看 CPU、内存、磁盘、网络等服务器状态
- 需要在多台服务器之间切换

典型需求：

```text
部署项目
查看日志
修改配置
管理文件
重启服务
查看服务器负载
```

---

## 6.2 初级开发者 / 学生

特点：

- 会基本 SSH
- 不熟悉复杂 SSH 参数
- 不理解 Host Key、Jump Host 等高级概念
- 更依赖图形化界面

核心诉求：

> 能快速连接服务器，不需要先学习复杂 SSH 知识。

---

## 6.3 独立开发者 / 小团队

特点：

- 没有专职运维人员
- 一个人可能同时负责开发、部署、数据库和服务器
- 希望减少软件数量

典型工具组合：

```text
Terminal
SFTP Client
Docker CLI
Database GUI
Monitoring Tool
```

MauLink 的长期目标是减少这些工具之间的切换。

---

## 6.4 轻量运维用户

特点：

- 管理多台服务器
- 需要快速查看状态
- 需要 SSH、SFTP、日志和进程信息
- 不一定需要大型监控平台

---

# 7. 非目标用户

MauLink 第一阶段不主要面向：

- 大型企业运维中心
- 大规模 Kubernetes 集群管理员
- 网络设备管理人员
- 需要 RDP/VNC 为核心场景的用户
- 需要复杂团队权限体系的企业用户
- 需要堡垒机审计能力的组织

这些方向未来可重新评估，但不属于当前产品核心。

---

# 8. 核心用户场景

## 8.1 首次连接服务器

用户流程：

```text
启动 MauLink
→ Add Server
→ 输入 Host
→ 输入 Username
→ 选择 Password / SSH Key
→ Test Connection / Connect
→ 首次 Host Key 确认
→ 进入 Server Workspace
```

目标：

> 新用户无需阅读说明文档即可完成第一次 SSH 连接。

---

## 8.2 日常服务器操作

用户进入服务器后：

```text
打开 Terminal
→ 执行命令
→ 查看服务器状态
→ 浏览服务器文件
→ 上传 / 下载文件
```

整个过程尽可能不离开当前 Workspace。

---

## 8.3 查看服务器性能

用户希望快速知道：

- CPU 是否过高
- 内存是否不足
- 磁盘是否快满
- 网络是否异常
- Load 是否异常
- 哪个进程占用最多资源

MauLink 应在不增加过多复杂度的情况下给出清晰信息。

---

## 8.4 多服务器管理

用户拥有多台服务器：

```text
Production
├── Web-01
├── Web-02
└── DB-01

Development
├── Dev-01
└── Dev-02
```

需要：

- 分组
- 搜索
- 快速连接
- 状态识别
- 快速切换

---

## 8.5 大文件传输

用户上传：

```text
1GB
10GB
甚至更大文件
```

MauLink 不应因为文件较大而出现明显内存暴涨或 UI 卡死。

---

# 9. 产品信息架构

第一阶段信息架构：

```text
MauLink
├── Servers
│   ├── Groups
│   ├── Search
│   └── Add Server
│
├── Server Workspace
│   ├── Overview
│   ├── Terminal
│   ├── Files
│   └── Monitor
│
└── Settings
    ├── General
    ├── Appearance
    ├── Terminal
    ├── Language
    └── Advanced
```

后续扩展：

```text
Server Workspace
├── Docker
└── Database
```

---

# 10. MVP 范围

MVP 定义为：

> 用户能够通过 MauLink 完成从服务器配置、SSH 登录、Terminal 操作、SFTP 文件管理到服务器性能查看的完整基础工作流。

MVP 必须包含：

1. Server Manager
2. SSH Password Login
3. SSH Key Login
4. Host Key Verification
5. Terminal
6. Multi Terminal
7. SFTP / File Manager
8. Server Monitor
9. Settings
10. Theme
11. Chinese / English
12. Credential Secure Storage
13. Windows / macOS
14. 基础错误处理
15. 基础性能优化

---

# 11. 功能优先级

采用：

- P0：必须实现，否则不能发布
- P1：重要功能，应尽快实现
- P2：增强功能
- P3：未来规划

---

# 12. Server Manager

**优先级：P0**

## 12.1 新增服务器

字段：

- Name
- Host
- Port
- Username
- Authentication Type

Authentication：

- Password
- SSH Key

默认端口：

```text
22
```

Port 默认不需要占据主要视觉位置，可放在 Host 输入区域或 Advanced。

---

## 12.2 编辑服务器

用户可修改：

- Name
- Host
- Port
- Username
- Authentication
- Group
- Advanced Settings

---

## 12.3 删除服务器

删除前必须二次确认。

删除服务器配置时应明确是否同时删除保存的凭据。

---

## 12.4 服务器分组

**优先级：P0**

支持：

```text
Production
Development
Personal
Other
```

用户可：

- 新增分组
- 删除分组
- 修改分组名称
- 调整服务器分组

---

## 12.5 搜索

**优先级：P0**

支持按：

- Server Name
- Host
- Group

快速过滤。

搜索必须实时响应。

---

## 12.6 Server List

服务器项至少展示：

```text
Name
Host / Description
Connection State
```

状态：

```text
Disconnected
Connecting
Connected
Reconnecting
Failed
```

不要在列表中默认加入大量实时图表。

---

# 13. SSH 连接

**优先级：P0**

支持：

- Password
- SSH Private Key

连接状态需要向用户明确展示。

---

## 13.1 Password Login

用户可选择：

```text
Remember Password
```

密码不得以明文形式存入普通配置数据库。

---

## 13.2 SSH Key

支持选择本地 Private Key。

第一阶段至少考虑常见格式。

如果 Key 带 Passphrase，应允许用户输入并选择是否安全保存。

---

## 13.3 Host Key Verification

**优先级：P0**

首次连接必须提示：

```text
首次连接到此服务器

Server:
192.168.1.20

Fingerprint:
SHA256:...

Trust & Connect
Cancel
```

文案需解释：

> 保存该指纹后，如果服务器身份以后发生变化，MauLink 会进行提醒。

禁止简单展示：

```text
yes / no
```

---

## 13.4 Host Key Changed

如果 Host Key 与已保存记录不一致：

必须使用高风险警告。

提示用户可能原因：

- 服务器系统重装
- SSH Key 重新生成
- Host/IP 被重新分配
- 中间人攻击风险

禁止默认自动接受。

---

## 13.5 自动重连

**优先级：P1**

网络临时中断时：

- 显示 Reconnecting
- 自动尝试有限次数
- 允许用户取消
- 失败后提供手动重连

不能无限后台重连。

---

# 14. Terminal

**优先级：P0**

Terminal 是 MauLink 的核心能力之一。

目标：

> 输入跟手、输出流畅、长时间使用稳定。

---

## 14.1 基本能力

支持：

- ANSI Color
- Unicode
- 中文
- IME
- Copy
- Paste
- Select
- Search
- Resize
- Scrollback
- Mouse
- Keyboard Shortcuts

---

## 14.2 中文体验

必须重点测试：

- 中文输入法
- 中文粘贴
- 中英文混合输入
- 中文字符宽度
- Emoji
- CJK 字符对齐

Windows 和 macOS 都必须单独测试。

---

## 14.3 Multi Terminal

**优先级：P0**

同一服务器支持多个 Terminal Tab。

例如：

```text
Terminal 1
Terminal 2
Terminal 3
+
```

支持：

- 新建
- 关闭
- 重命名（P1）
- 快速切换

---

## 14.4 Scrollback

默认：

```text
10,000 lines
```

用户可配置。

禁止无限 Scrollback。

---

## 14.5 Terminal Search

**优先级：P1**

支持当前 Terminal 内容搜索。

---

## 14.6 快捷键

至少支持：

```text
New Terminal
Close Terminal
Find
Copy
Paste
Clear
Reconnect
```

Windows 使用 Ctrl 体系。

macOS 使用 Command 体系。

---

# 15. SFTP / File Manager

**优先级：P0**

目标：

> 用户连接服务器后，不再需要单独打开其他 SFTP 软件。

---

## 15.1 基础浏览

支持：

- 目录浏览
- 返回上级
- Path Navigation
- Refresh
- 文件 / 目录区分
- Size
- Modified Time

---

## 15.2 文件操作

必须支持：

- Upload
- Download
- Rename
- Delete
- New Directory
- Copy Path

---

## 15.3 Drag & Drop

**优先级：P1**

支持从 Finder / Explorer 拖入文件上传。

---

## 15.4 文件传输进度

必须展示：

- File Name
- Progress
- Speed
- Remaining
- Status

状态：

```text
Waiting
Uploading
Downloading
Completed
Failed
Cancelled
```

---

## 15.5 传输队列

**优先级：P1**

支持多个文件任务。

可：

- Cancel
- Retry
- Clear Completed

---

## 15.6 大文件要求

必须采用流式传输。

产品表现要求：

> 上传 1GB、10GB 等大文件时，内存不应随文件大小线性增长。

---

# 16. Server Monitor

**优先级：P0**

Server Monitor 是 MauLink 的核心差异化能力之一。

设计目标：

> 比普通 SSH 工具更详细，比专业运维平台更容易理解。

---

# 17. Overview

默认显示：

- CPU
- Memory
- Disk
- Network
- Load
- Uptime

用户不需要进入复杂页面即可快速判断服务器状态。

---

## 17.1 CPU Card

至少展示：

- Usage
- Historical Trend
- Core Count
- Load

Detailed 中展示：

- User
- System
- IO Wait

---

## 17.2 Memory Card

至少展示：

- Used
- Total
- Usage %
- Historical Trend

Detailed：

- Available
- Cache
- Buffer
- Swap

---

## 17.3 Disk Card

至少展示：

- Main Filesystem Usage
- Used
- Total

Detailed：

- Partition
- Mount
- Filesystem
- Read
- Write
- IOPS

---

## 17.4 Network Card

至少展示：

- Current Download
- Current Upload
- Historical Trend

Detailed：

- Interface
- Total RX
- Total TX

---

## 17.5 System Information

展示：

- Hostname
- OS
- Kernel
- Architecture
- CPU Model
- Uptime

---

# 18. Process Monitor

**优先级：P1**

至少支持：

- PID
- Process Name
- CPU
- Memory
- User
- Command

排序：

- CPU
- Memory

第一阶段仅查看。

Process Kill 属于后续增强能力。

---

# 19. Monitor UX

监控页面不得设计成复杂运维大屏。

原则：

- 一眼看懂
- 信息有层级
- 图表简单
- 实时但不过度刷新
- 当前服务器优先
- 非当前服务器降低刷新频率

---

# 20. Adaptive Sampling

产品必须具备智能降低后台监控成本的能力。

当前服务器：

```text
CPU        约 1s
Network    约 1s
Memory     约 2s
Disk IO    约 2s
Process    约 3~5s
Filesystem 约 10s
```

后台服务器：

降低刷新频率。

窗口最小化：

大幅降频或暂停非必要指标。

具体实现策略在技术设计中确定。

---

# 21. Settings

**优先级：P0**

至少包含：

```text
General
Appearance
Terminal
Language
Advanced
```

---

## 21.1 Language

支持：

- 简体中文
- English

切换后应立即生效，尽可能避免要求重启。

---

## 21.2 Theme

支持：

```text
System
Light
Dark
```

---

## 21.3 Terminal Settings

至少：

- Font Family
- Font Size
- Scrollback
- Cursor Style
- Terminal Theme（P1）

---

# 22. UI / UX 需求

## 22.1 整体风格

目标：

- 极简
- 现代
- 专业
- 易读
- 稳定
- 克制

避免：

- 传统后台管理页面感
- 过度渐变
- 大量边框
- 过度毛玻璃
- 高频复杂动画
- 过度装饰

---

## 22.2 动效

动效用于：

- 页面切换
- 展开 / 收起
- Hover
- Loading
- 状态变化

建议：

```text
150~250ms
```

不得因为视觉效果明显影响性能。

---

## 22.3 信息密度

界面允许较高信息密度，但必须通过：

- 间距
- 字体层级
- 分区
- 图标
- 对齐
- 色彩

保证可读性。

---

# 23. 错误提示

错误信息必须面向普通用户。

禁止直接展示：

```text
ECONNREFUSED
IO Error 10061
AuthenticationFailure
```

主提示示例：

```text
无法连接服务器

192.168.1.20:22

可能原因：
• 服务器未启动
• SSH 服务未运行
• 端口配置错误
• 网络或防火墙阻止连接

查看详细错误
```

技术错误放在：

```text
Details
```

中展示。

---

# 24. Loading / Empty / Error State

所有主要页面必须设计：

- Loading State
- Empty State
- Error State

禁止空白页面。

例如服务器列表为空：

```text
还没有服务器

添加第一台服务器即可开始使用 MauLink。

[ Add Server ]
```

---

# 25. 安全需求

## 25.1 凭据存储

密码、Passphrase、Token 等敏感数据不得明文存入普通 SQLite 数据库。

应使用操作系统安全存储。

---

## 25.2 Private Key

Private Key 默认不复制进入应用数据库。

保存其路径或安全引用。

---

## 25.3 日志

应用日志禁止输出：

- Password
- Private Key Content
- Passphrase
- Database Password
- Token

---

## 25.4 Host Key

必须保存并校验已信任服务器 Host Key。

---

# 26. 性能需求

性能是 MauLink 的产品核心指标之一。

以下属于当前工程目标，最终以真实设备 benchmark 为准。

---

## 26.1 Cold Start

目标：

```text
≤ 1.5 ~ 2s
```

---

## 26.2 Idle CPU

目标：

```text
接近 0%
```

正常空闲：

```text
目标 < 1%
```

---

## 26.3 Memory

基础空闲窗口目标：

```text
尽可能控制在约 100MB 内
```

考虑不同系统 WebView 差异，该数值作为优化目标，不作为单个平台绝对承诺。

---

## 26.4 UI FPS

正常：

- 页面滚动
- Tab 切换
- Terminal 输入
- Server List
- Monitor

目标：

```text
60 FPS
```

不得出现明显卡顿。

---

## 26.5 Terminal Latency

用户输入后应即时反馈。

不能因为：

- Monitor
- SFTP
- 后台任务

导致明显输入延迟。

---

## 26.6 Large Output

场景：

```text
docker logs -f
yes
large log output
```

要求：

- UI 不冻结
- 内存不无限增长
- Terminal 仍可响应
- 可中断

---

## 26.7 Multi Session

至少验证：

```text
1
5
10
20
```

SSH Session。

需要关注：

- RAM
- CPU
- Terminal Input
- Monitor Overhead

---

# 27. 内存控制需求

以下数据必须设置容量边界：

- Terminal Scrollback
- Monitor History
- Log Buffer
- Transfer Queue
- Process History
- Event Queue

禁止无限增长。

---

# 28. 后台任务需求

任何后台任务都必须回答：

1. 为什么需要持续运行
2. 多久执行一次
3. 用户不可见时是否可以暂停
4. Window Minimized 时是否可以降频
5. Session Disconnect 后是否立即停止

原则：

> Zero Unnecessary Background Work

---

# 29. 可访问性与可读性

第一阶段至少考虑：

- 足够字体对比度
- 清晰 Focus State
- Keyboard Navigation
- UI 不只依赖颜色表达状态
- 状态图标 + 文案

例如：

```text
● Connected
```

而不能只显示绿色圆点。

---

# 30. Windows / macOS 一致性

必须保证：

- 功能一致
- 数据格式一致
- 配置一致
- Workspace 结构一致
- Server Profile 可迁移
- 主要快捷键符合各自平台习惯

允许原生系统差异：

```text
Windows Credential Manager
macOS Keychain
```

---

# 31. 数据与配置

第一阶段需要保存：

- Server Profiles
- Groups
- Settings
- UI Preferences
- Terminal Preferences
- Non-sensitive History

敏感信息单独使用系统安全存储。

---

# 32. 数据导入 / 导出

**优先级：P2**

后续支持：

- Export Server Config
- Import Server Config

导出文件默认不能包含明文密码。

---

# 33. Advanced SSH

以下属于后续版本：

**优先级：P2**

- Jump Host
- Port Forward
- SOCKS Proxy
- HTTP Proxy
- SSH Agent
- Advanced Algorithm Configuration

第一阶段架构应允许未来加入，但不作为 MVP 发布阻塞项。

---

# 34. Docker 管理

**优先级：P3 / 后续版本**

Docker 是 MauLink 的重要长期扩展方向，但不进入当前 MVP。

计划包含：

```text
Containers
Images
Volumes
Networks
Compose
Logs
Terminal
Inspect
```

用户连接服务器后可直接查看 Docker 环境。

---

## 34.1 Containers

展示：

- Name
- Status
- Image
- CPU
- Memory
- Network
- Ports

支持：

- Start
- Stop
- Restart

Delete 等危险操作需要明确确认。

---

## 34.2 Docker Logs

默认只读取最近一定数量的日志。

例如：

```text
Last 500 Lines
```

之后通过流式方式继续读取。

禁止一次加载全部历史日志。

---

# 35. Database 管理

**优先级：P3 / 后续版本**

初期规划：

- MySQL
- PostgreSQL
- Redis
- SQLite

优先考虑通过现有 SSH Session / SSH Tunnel 连接远程数据库。

---

## 35.1 Database Workspace

未来包含：

```text
Connections
Schemas
Tables
Data
SQL Editor
Query History
```

---

## 35.2 Large Result

数据库查询结果不得一次性渲染海量数据。

必须使用：

- Pagination
- Virtual Table

后续可增加：

- Streaming Result

---

# 36. 第一阶段明确不做

为避免范围失控，当前版本不包含：

- RDP
- VNC
- FTP
- Telnet
- Serial
- Kubernetes
- Cloud Provider Management
- Team Cloud Sync
- Enterprise RBAC
- Bastion Audit
- AI Agent
- Remote Desktop
- 自定义 Server Agent

---

# 37. MVP 成功标准

MVP 可以进入 Beta 的最低条件：

## 功能

- 能稳定添加服务器
- Password 登录稳定
- SSH Key 登录稳定
- Host Key 校验正确
- Terminal 可长期使用
- 中文输入正常
- Multi Terminal 正常
- SFTP 基本操作完整
- 文件上传下载稳定
- Monitor 数据正确
- 中英文可切换
- Light / Dark 可用
- Windows 与 macOS 均可运行

---

## 体验

新用户能够在不阅读文档的情况下：

```text
启动 MauLink
→ 添加服务器
→ 成功连接
→ 打开 Terminal
→ 上传文件
→ 查看服务器 CPU / Memory
```

---

## 性能

必须通过：

- Cold Start Test
- Idle CPU Test
- Idle Memory Test
- Terminal Stress Test
- SFTP Large File Test
- Multi Session Test
- Long Running Test

---

# 38. 版本规划

## v0.1 — Foundation

目标：

> 建立产品骨架和基础 SSH 能力。

包含：

- Tauri Desktop Shell
- Vue UI
- Theme
- i18n
- Server Manager
- Password SSH
- SSH Key
- Basic Terminal
- Basic Workspace

---

## v0.2 — Files & Sessions

包含：

- SFTP
- File Manager
- Multi Terminal
- Host Key
- Credential Storage
- Settings
- Reconnect

---

## v0.3 — Monitor

重点：

> 建立 MauLink 的核心差异化监控体验。

包含：

- CPU
- Memory
- Disk
- Network
- Load
- Uptime
- System Information
- Historical Chart
- Process View

---

## v0.4 — UX & Performance

重点：

- UI 细节
- Shortcut
- Search
- Command Palette
- Animation
- Virtualization
- Performance
- Memory Optimization
- Long-running Stability

---

## v0.5 — Advanced SSH

包含：

- Jump Host
- Port Forward
- Proxy
- SSH Agent
- Advanced Authentication

---

## v0.6 — Docker

包含：

- Containers
- Images
- Networks
- Volumes
- Compose
- Logs
- Container Terminal

---

## v0.7 — Database

包含：

- MySQL
- PostgreSQL
- Redis
- SQLite
- SSH Tunnel
- SQL Editor
- Data Browser

---

## v1.0 — Stable Release

目标：

- 完整稳定性测试
- 正式安装包
- 自动更新
- Crash Handling
- Documentation
- Signature
- Public Release

---

# 39. 验收原则

每个功能必须同时通过四类验收：

```text
Functional
UX
Performance
Security
```

不能仅因为：

```text
“功能可以运行”
```

就判定完成。

例如 Server Monitor 完成标准不是：

```text
可以看到 CPU 数据
```

而是：

```text
数据正确
+
界面清晰
+
刷新平滑
+
不明显影响 CPU
+
后台自动降频
```

---

# 40. 产品核心指标

MauLink 当前阶段最关注：

## 使用体验

- Add Server 完成率
- 首次连接成功率
- 基础操作步骤数
- 错误提示理解成本
- Terminal 操作流畅度

## 稳定性

- Crash Rate
- Connection Failure
- Memory Leak
- Long-running Stability

## 性能

- Cold Start
- Idle CPU
- Idle Memory
- Terminal Latency
- SFTP Throughput
- Monitor Overhead

Beta 阶段再决定是否加入匿名 Telemetry。

默认不应强制收集用户隐私数据。

---

# 41. 隐私原则

MauLink 应尽可能采用：

> Local First

服务器配置主要存储在本地。

默认不上传：

- Host
- Username
- Password
- Private Key
- Terminal Content
- Server Files
- Command History

未来如果增加 Cloud Sync，必须：

- 明确告知
- 用户主动开启
- 独立隐私策略
- 敏感数据加密

---

# 42. 产品命名规范

正式名称：

```text
MauLink
```

推荐英文品牌语：

> A modern server workspace.

中文定位：

> 简洁、高效的开发者服务器工作台。

建议 GitHub Repository：

```text
maulink
```

应用显示名称：

```text
MauLink
```

---

# 43. PRD 需求冻结规则

进入开发后：

P0 需求变更必须记录。

任何新增需求需要明确：

- Priority
- Version
- User Value
- UX Impact
- Performance Impact
- Security Impact

禁止开发过程中无边界追加功能。

---

# 44. 后续文档

PRD 确认后，按照以下顺序继续产出：

```text
1. MauLink PRD
        ↓
2. UX / Information Architecture
        ↓
3. UI Design System
        ↓
4. System Architecture
        ↓
5. Technical Design
        ↓
6. API / IPC Design
        ↓
7. Development Plan
        ↓
8. Test Plan
        ↓
9. Release Plan
```

---

# 45. 当前结论

MauLink 第一阶段必须专注于：

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
优秀 UX
+
低资源占用
```

Docker 和 Database 属于后续重要扩展方向，但不得影响 MVP 聚焦。

MauLink 第一阶段成功的判断标准不是：

> 功能很多。

而是：

> 一个第一次使用 MauLink 的用户，可以快速完成服务器连接、终端操作、文件管理和服务器状态查看，并且整个应用保持美观、流畅、稳定、低资源占用。

这将作为 MauLink 当前阶段的产品需求基线。
