# 终端与 SFTP 区域紧凑化改造记录

## 实施结果

- 内嵌文件区默认占终端主区域 30%，最高 42%；标准密度最小 160px，紧凑密度最小 148px。终端至少保留 240px，分隔条为 6px。
- 分隔条支持鼠标拖动、键盘 `↑` / `↓` 每次调整 12px、`Home` / `End` 调到边界，双击恢复 30%。鼠标或键盘操作结束时才保存布局。
- 展开状态和高度按稳定的 `serverId` 写入 `maulink.workspace.layout.v1`。窗口空间不足时暂时隐藏内嵌区且不改保存偏好，文件按钮改为打开独立文件页；专注模式同样不改偏好。
- 终端与 SFTP 仍使用原有组件和状态流；没有调整 Rust 契约、AppSettings、SFTP 游标分页、文件传输、目录同步或 xterm 实例。
- 文件位置、路径编辑和祖先目录菜单合并到一行。窄文件区隐藏修改时间列，并将新建文件夹放入“更多操作”；下载及查看/编辑入口也在此菜单中。文件行保留上下文菜单。
- 表头和数据行分别按标准密度 30px / 36px、紧凑密度 28px / 32px 呈现。第一页没有后续游标时不显示分页；末页不显示“下一页”。

## 改造前后测量

以下尺寸由可交互 Web 原型的浏览器 DOM 实测，单位为 CSS px。终端主区高度取相同视口下 `.terminal-main` 的实际高度。

| 视口 | 主区 | 改造前：终端 / 文件区 | 改造后：终端 / 分隔条 / 文件区 | 工具栏 / 面包屑 | 表头 / 行 | 分页 |
|---|---:|---:|---:|---:|---:|---|
| 860×640 | 414 | 230 / 184（文件区 44.4%） | 248 / 6 / 160 | 54 + 22 / 39 + 0 | 33 / 40 → 30 / 36 | 29px → 首屏隐藏 |
| 988×756 | 530 | 294 / 236（文件区 44.5%） | 364 / 6 / 160 | 54 + 22 / 39 + 0 | 33 / 40 → 30 / 36 | 29px → 首屏隐藏 |
| 1440×920 | 694 | 386 / 308（文件区 44.4%） | 480 / 6 / 208 | 54 + 22 / 39 + 0 | 33 / 40 → 30 / 36 | 29px → 首屏隐藏 |

紧凑密度在 860×640 下实测文件区 148px、表头 28px、数据行 32px。独立文件页占满终端主区。窄屏检查器开启时，文件面板宽 434px；修改时间列隐藏，新建文件夹移入菜单，文档无横向溢出。

## 浏览器证据

改造前和改造后均覆盖 860×640、988×756、1440×920，每个视口包含终端、内嵌文件区和独立文件页。每张截图均独立重载两次并得到相同 SHA-256；9 张改造后截图均记录 `overflow: false`。

| 视口 | 改造前 | 改造后 |
|---|---|---|
| 860×640 | [before 终端截图与清单](evidence/before/capture-light-zh-CN-860x640-terminal.json)、[文件区与独立文件页截图及清单](evidence/before/capture-light-zh-CN-860x640-files_inline-files.json) | [after 截图与清单](evidence/after/860x640/capture-light-zh-CN-860x640-terminal_inline-files_files.json) |
| 988×756 | [before 截图与清单](evidence/before/capture-light-zh-CN-988x756-terminal_inline-files_files.json) | [after 截图与清单](evidence/after/988x756/capture-light-zh-CN-988x756-terminal_inline-files_files.json) |
| 1440×920 | [before 截图与清单](evidence/before/capture-light-zh-CN-1440x920-terminal_inline-files_files.json) | [after 截图与清单](evidence/after/1440x920/capture-light-zh-CN-1440x920-terminal_inline-files_files.json) |

另有 [紧凑密度截图与清单](evidence/after/compact/860x640/capture-light-zh-CN-860x640-inline-files.json)。

## 验收

- 浏览器：三个方案视口的 9 张截图全部通过双次重载一致性检查；另检查了 434px 窄文件面板、菜单、无横向溢出、短窗口转独立文件页、恢复窗口后恢复内嵌布局、键盘调节、鼠标拖动、双击复位和专注模式恢复。
- 单元测试：`npm --prefix frontend run test`，33 个测试文件、327 个测试全部通过。
- 类型检查：`npm --prefix frontend run type-check` 通过。
- 生产前端构建：`npm --prefix frontend run build` 通过。
- 补丁空白检查：`git diff --check` 通过。

## 验收边界

当前 Codex 内嵌浏览器环境未提供 `localStorage` API。实现对此做了异常安全回退；按服务器隔离、比例范围、损坏数据和存储不可用由 `workspace-layout.test.ts` 覆盖。真实 Tauri WebView 中的跨重载偏好持久化仍需在应用运行环境复核。

本次没有连接真实 SSH/SFTP 主机；验证使用项目的确定性 Web Mock IPC。Windows/macOS 原生 SFTP、真实远端游标生命周期和签名安装包未由本报告中的 Web 验收覆盖。
