# MauLink 服务器外观统一改造实施记录

## P0：基线与范围

- 日期：2026-10-12。
- 设计审查基线：`bb0ccf8dcdc79a5e57d0f5b2034503ba48af65fc`，与 `main` / `origin/main` 相同。
- 实际实施基线：`42b7919aa2637f5827166f1014f633bde913f8b1`，分支 `dev/app-updates`；`main...HEAD` 为 `0 2`，领先提交是已交付的 Quick Monitor 更新。
- 起始工作区：干净。
- 分支偏差：手册要求从 `main` 新建 `feature/server-appearance-editor`，但用户已明确要求所有 MauLink 更新继续在现有 `dev/app-updates` worktree 完成、不新开分支。因此保留当前 worktree 与分支，提交后推送该分支，不创建 PR。
- 初始验证：`npm --prefix frontend run type-check` 通过。`npm --prefix frontend test` 为 27 个测试文件、263 项：26 个文件通过、258 项通过、`settings-workspace.test.ts` 中 5 项失败；这些断言仍期待设置页已移除的“保存更改”按钮和离页对话框，和当前自动保存实现不匹配。
- 依赖：使用当前 lockfile 已安装依赖；未运行 `npm ci`，避免在验证可用的依赖树上进行无必要的删除重装。
- 计划文件：服务器弹窗与外观面板；共享终端外观控件和预览；全局终端设置适配；样式与中英文消息；服务器/控件/设置/主题测试；本实施记录。

## 实施阶段

| 阶段 | 状态 | 记录 |
|---|---|---|
| P0 基线 | 已完成 | 见上文。 |
| P1 公共控件 | 已完成 | 新增 BaseSlider、BaseColorPicker、TerminalBackgroundPositionPicker、TerminalThemeModeSelector；组件测试覆盖范围、禁用、键盘、HEX 校验和 9 种位置。 |
| P2 外观布局与预览 | 已完成 | 新增共享编辑器、预览和服务器外观面板；环境移至高级配置；通过 860×640 与 1440×920、浅/深/紧凑条件下的 Web 检查。 |
| P3 统一保存与图片生命周期 | 已完成 | 底部唯一保存按 dirty 阶段写 Profile / Appearance；部分成功只重试失败阶段；取消确认、图片引用安全清理和过期选择回调清理已实现并测试。两个 IPC 不构成数据库级原子事务。 |
| P4 全局复用与回归 | 已完成 | 全局设置改用共享编辑器，保留 350ms 自动保存及 `discard/commit/validate` 资源流程；完成完整前端回归和视觉截图。 |

## 最终验证与偏差

- 分支按用户指示继续使用 `dev/app-updates`，未按手册新建 `feature/server-appearance-editor`；提交推送后不创建 PR，由用户处理。
- 最终验证（2026-10-12）：`npm run type-check` 通过；`npm test` 通过，28 个测试文件、272 项；`npm run build` 通过。构建有既存/常见的大型 JavaScript chunk 提示（980.64 kB，gzip 283.17 kB），未阻断构建。Vitest 输出 jsdom Canvas 未实现及 localStorage 实验性警告，相关测试均通过。
- Web Mock IPC 验收覆盖服务器外观弹窗浅/深主题、继承全局背景图、背景图模式无图时阻止保存、颜色控件、图片模式控件、英文界面/红色强调色，以及 860×640 紧凑窗口固定操作区。截图捕获均重复两次，哈希一致；实际 CSS 视口、主题、密度、语言、DPR=1、字体加载及横向溢出已核验。
- 截图与 manifest：`docs/refactor/screenshots/server-appearance-editor-20261012/`，包含浅/深自定义颜色、浅/深背景图设置、服务器继承全局背景图、860×640 紧凑窗口、英文红色强调色共 7 组。
- Rust 源码未改动，Rust 测试未涉及。浏览器 Web 验证符合用户当前交付要求；Windows/Tauri 未验证。构建后会再进行 macOS 应用重建。
- 剩余交付：在 `dev/app-updates` 提交并推送，然后重建 macOS 应用；不创建 PR。
