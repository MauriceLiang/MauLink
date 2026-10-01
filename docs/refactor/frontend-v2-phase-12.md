# Phase 12 验收报告：正式切换 Vue 前端

检查日期：2026-10-01（Asia/Shanghai）。切换前恢复点：main `f75d21a`（Phase 11）。

## 根因与修改内容

迁移工程已经具备 Server/Group CRUD、SSH 与安全挑战、Terminal、SFTP/Transfer、共享 Monitor、Settings、i18n、Palette、主题与视觉回归；默认 Tauri 仍加载旧静态 frontend。本阶段将 Vue 工程正式切换为 `frontend/`，默认桌面改为 Vite dev/build output。

用户持续授权后续阶段采用 Browser 验收门禁，并曾单独授权保留 Phase 6 缺口继续。本阶段按该范围执行，**PASS（自动化 + Browser + macOS build/startup）**；没有把未完成的 Native/Windows/Phase 6 检查改写为通过，也没有宣称双平台发布验收完成。

- `frontend-v2/` 移至 `frontend/`；旧入口、.mjs、legacy CSS 和 vendored assets 从活动工程删除。完整旧代码仍在 Git `f75d21a:frontend/`，本机另保留 `/private/tmp/maulink-phase12-legacy-frontend-f75d21a` 副本；临时目录不是长期恢复来源。
- 正式 `tauri.conf.json` 使用 devUrl、npm hooks、`../frontend/dist`，关闭 withGlobalTauri。正式 identifier、窗口尺寸/Overlay/交通灯位置、production CSP 不变；仅 main 继承已验收的 start-dragging 权限，devCsp 额外允许本机 HMR。
- 保留隔离资料配置 `tauri.frontend-v2.conf.json`，同样加载当前 frontend/dist，并继承正式窗口权限。
- 静态 `tauri.harness.conf.json` 清除继承的 Vite hooks/devUrl，单独保留全局 API，防止产品切换破坏现有开发调试页。产品窗口仍用模块化 @tauri-apps/api。
- npm 工程名改为 maulink-frontend，index title 改为 MauLink；install 仅更新 lockfile 工程名，依赖解析完全不变。TypeScript 仍精确 5.9.3。
- 更新 README、frontend 开发/视觉回归命令、RustRover Run Config 文档、design-qa Page 19，新增迁移总结与本报告。

## 修改文件

```text
frontend-v2/* → frontend/*
旧 frontend/{src/*.mjs,tests/*.mjs,vendor/*,styles.css,index.html} 移除/替换
frontend/{package.json,package-lock.json,index.html,README.md,tests/shell.test.ts,visual/README.md}
src-tauri/tauri.conf.json
src-tauri/tauri.frontend-v2.conf.json
src-tauri/tauri.harness.conf.json
.gitignore / README.md / design-qa.md
docs/refactor/frontend-v2-phase-12.md
docs/refactor/frontend-migration-summary.md
docs/refactor/screenshots/phase-12/*
```

Rust Core、Tauri commands、数据库 schema、contracts/v1、主业务 capability 和 Cargo.lock 未修改；模块相对路径层级相同，IPC 类型/协议不迁移。

## 自动化检查

| 检查 | 结果 |
| --- | --- |
| npm --prefix frontend install --offline --no-audit --no-fund | PASS，up to date；缓存安装，未升级依赖 |
| npm run type-check | PASS |
| npm run test | PASS，11 files、126/126 tests |
| npm run build | PASS，126 modules；JS 507.59 kB / gzip 143.51 kB，CSS 29.39 kB |
| production JS/CSS 与切换前 hash | 相同：index-BqGkWg3h.js / index-BHTPZV0f.css；DEV fixtures 排除 |
| baseline 保护 Node test | PASS，1 |
| 视觉 manifest / hash / JPEG 校验 | PASS，184 implementation + 160 reference |
| cargo fmt --all -- --check | PASS |
| cargo check --workspace --all-targets --locked | PASS |
| cargo clippy --workspace --all-targets --locked -- -D warnings | PASS |
| cargo test --workspace --locked | PASS，68 passed、0 failed、11 ignored；未重跑 ignored SSH / 大文件压力测试 |
| 正式 Tauri dev --no-watch -- --locked | PASS 启动；自动运行 frontend npm run dev，Vite 127.0.0.1:1420；cargo dev 编译并运行 target/debug/maulink，随后停止 |
| 正式 Tauri build --bundles app --ci -- --locked | PASS；npm hook 自动构建，Rust release 26.04s，1 app bundle，19.88 MiB |
| ad-hoc 整包签名及 codesign --verify --deep --strict | PASS（本地测试用途，非 Developer ID / 公证） |
| Core / contracts 完整性、依赖解析、git diff --check | PASS |

初次测试 124 passed / 1 failed：拖动回归仍读旧隔离配置，切换后权限由正式配置继承。将该测试改为验证正式 main 权限及隔离继承，另增加产品/静态 IPC Harness 入口回归，重新全部通过；没有删除替代行为验收。

初次 bundle 签名校验失败：linker 的签名缺 bundle resource seal；对本地 app 执行 codesign --force --sign - 后整包验证通过。npm fsevents install-script 审批提示、Vitest jsdom Canvas/localStorage 提示、Vite >500 kB chunk 提示仍存在；未放宽规则或阈值掩盖。

## Browser 实测

切换后从当前 frontend 的 Vite 服务执行真实 UI recipe，23 个中文浅色 1440×920 页面全部与 Phase 10 原基线一致；每页独立 reload 两次，默认 update=false，不覆盖基线。覆盖首页、CRUD、SSH 安全/认证/失败、Terminal/Focus、Files/Delete/Transfer、Monitor 质量、Settings、Palette、Menu、Toast。

双语言/双主题及四视口证据保留 Phase 10 / 11；本次没有样式或业务行为变化，production JS/CSS hash 相同。追加首页和活动终端截图至 phase-12，Shell ready、110×33、Quick Monitor 与实际焦点可见。这是 Typed Mock Browser 证据，不是真实 SSH/原生 WebView 截图。

## Tauri 实测

本阶段实际启动正式 Tauri dev 进程并构建正式 macOS arm64 app；未代用户操作应用界面。用户上一轮已确认独立 Vue release 的 1–7 项操作正常，包含真实 SSH/终端、快捷键、菜单、设置、语言/主题、窄窗口、拖动与重启保存。本轮正式 identifier 的实际 GUI / 老资料加载未再次手动验证，按用户已取消的 Native 门禁记录为未实测。

产物：`/Users/mauriceliang/Documents/code/MauLink/target/release/bundle/macos/MauLink.app`，CFBundleIdentifier `io.maulink.desktop`，版本 0.1.0，arm64。

ad-hoc 签名后二进制 SHA256：`54f8f403da9a74c310c68d3375705ad11505f5a7243c6c9407dfb485dc9cc060`。旧 2026-09-28 ZIP 未重建，不作为本轮交付。

## 剩余风险与退出结果

- 正式 identifier 使用原正式数据目录；没有复制隔离资料、删除用户数据或修改 schema。已有正式资料的 GUI 兼容性仍待用户实际启动确认。
- Windows WebView2、Credential Store、系统 picker/路径、installer、Native 最小化/恢复、真实 Linux 成功指标及 Phase 6 手动行为缺口仍未全部验收。默认 Rust ignored tests 不登记为本阶段 PASS。
- 远程文件 View/Edit 因既有 Core IPC 缺失保持禁用；未发现本次切换造成的新 P0/P1 blocker。
- 无 Developer ID / 公证或 Windows 签名，当前产物是本机开发测试包。
- **Phase 12 PASS（用户授权范围）**，提交推送 main；完整双平台发布 QA 继续 **BLOCKED**。前端迁移至此完成，可选 Phase 13 不属于本次前端迁移，等待正式应用稳定后另行决定。
