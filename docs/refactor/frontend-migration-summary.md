# MauLink Vue 前端迁移总结

更新：2026-10-01。当前正式入口为 `frontend/`：Vue 3 + TypeScript 5.9.3（精确版本）+ Vite + 模块化 Tauri API；默认 Tauri 加载 frontend/dist，开发自动启动本机 Vite/HMR。

## 已完成与证据

| 阶段 | 状态与范围 |
| --- | --- |
| 0–2 | 基线、稳定 TS/Vite/Tauri HMR、Typed/Mock IPC、Error Mapper、Base Components；Browser 与用户键盘/HMR 验收通过 |
| 3–4 | Shell、原生拖动修正、Server/Group CRUD；用户 Desktop 验收通过 |
| 5 | SSH、安全挑战与认证；按用户 Browser 门禁 PASS |
| 6 | xterm、Channel/ACK、标签与设置已接入；完整键盘/行为验收缺口仍 BLOCKED，用户明确授权先推送并进入后续阶段 |
| 7–8 | SFTP/Transfer、共享 Monitor、质量状态与历史；Browser 与相应 Core/隔离 OpenSSH 证据通过，未假设 Native/Windows PASS |
| 9 | Settings、中文/English、Palette/Menu；用户已在独立 macOS release 完成 1–7 项实测，包括真实 SSH、快捷键放行和重启持久化 |
| 10 | 23 实现状态、20 原型状态 × 双主题/双语/双视口；184+160 固定截图，重复 hash 相同；默认回归保护 baseline |
| 11 | Browser 四视口与活动终端 resize / 专注退出通过；16 张补充截图；Windows/完整 Native QA 未实测 |
| 12 | 正式入口切换、配置/开发文档/旧入口清理；126 frontend tests、68 Rust tests、Browser 原基线回归、macOS startup/build/signature 验证通过 |

阶段细节见 `docs/refactor/frontend-v2-phase-*.md`，视觉能力/平台缺口见根目录 `design-qa.md`。这些历史文件保留原阶段路径；新的开发命令以 frontend 为准。

## 运行

```bash
npm --prefix frontend ci
cargo tauri dev
# 停止开发服务后构建当前平台：
cargo tauri build --bundles app -- --locked
```

Browser 单独运行 `npm --prefix frontend run dev`；普通入口需要 Native IPC，开发测试用 `?harness=visual&page=servers&theme=light&locale=zh-CN` 等显式 Mock Harness。完整页面清单见 frontend/visual/cases.json，CUA 回归说明见 frontend/visual/README.md。

macOS 当前本地测试应用：`target/release/bundle/macos/MauLink.app`。正式标识 io.maulink.desktop 保持原资料目录；可选 `--config src-tauri/tauri.frontend-v2.conf.json` 使用隔离标识和同一个 Vue 工程。没有将隔离测试资料自动导入正式资料。

## 回退与边界

切换前 Git 恢复点为 `f75d21a`。可在独立 checkout/worktree 检出该提交，取得旧 frontend、正式旧配置和完整 frontend-v2；不应将旧目录和新 tauri.conf.json 混用。回退代码不会还原或删除本机用户资料。

Rust Core、IPC DTO/commands、contracts/v1、数据库 schema 与依赖解析未在切换阶段改变。生产 JS/CSS 与 Phase 9/10 相同；DEV Mock 不进入 release。

用户授权的 Browser 门禁下前端迁移完成，并已允许各通过阶段自动公开提交推送 main；这不等于完整发布认证。Phase 6 原缺口、Windows/Native 生命周期与系统服务未实测、既有远程 View/Edit 缺 Core 能力、性能指标/签名公证发行仍保留。当前应用是本机 ad-hoc 测试包，旧 ZIP 不是本轮产物。

可选 Phase 13 Rust/Tauri 大文件拆分不是本次前端迁移的一部分；必须等正式前端稳定后另行安排。本次未执行。
