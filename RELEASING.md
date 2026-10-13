# MauLink 构建与发布

本文说明 MauLink 的 PR 检查、跨平台安装包构建和自动 GitHub Release 流程。首期目标是 macOS Apple Silicon、macOS Intel 与 Windows x64。Packaging Smoke 在 PR 或手动触发时构建并检查三个平台的安装包，上传保留 7 天的测试产物；版本递增的 PR 合并到 `main` 后，发布工作流会自动构建并公开 Release。Linux、Windows ARM64、可信代码签名和公证不在本期支持范围。应用内更新界面与本地交互已接入，但没有真实更新地址和受信公钥，在线更新尚未启用；详见 [UPDATING.md](UPDATING.md)。

应用内更新功能 PR 不递增产品版本、不创建 Tag 或 Release。待真实更新地址、签名密钥托管和平台验收准备好后，由负责人另行安排版本递增发布；不要把本地构建或 Packaging Smoke 产物当作已公开的在线更新包。

## 首次启用

1. 在 GitHub 仓库确认 `Settings → Actions → General` 允许本仓库工作流运行，并查看 Actions 使用量、缓存和 14 天构建产物保留所产生的费用或配额影响。
2. 将 `CI` 工作流的以下状态检查设置为 `main` 的 required checks：
   - `Frontend / typecheck / unit / build`
   - `Rust Core / fmt / clippy / test`
   - `Desktop check / macos-15`
   - `Desktop check / windows-2025`
3. 首次 PR 成功运行后再设置 required checks；状态检查名称以 GitHub 展示的实际 job 名称为准。
4. PR 工作流只需要 `contents: read`。仅发布 job 获得 `contents: write`；它只在版本递增合入 `main` 后运行。不要使用 `pull_request_target` 执行 PR 代码，不要把发布 Token、证书或更新私钥传给 PR job。

工作流使用 GitHub-hosted `ubuntu-24.04`、`macos-15`、`macos-15-intel` 和 `windows-2025`。Action 使用 `actions/checkout@v7`、`actions/setup-node@v7`、`actions/upload-artifact@v7` 和 `actions/download-artifact@v8`；升级这些版本时先核对官方兼容说明及 Runner 要求。

## PR 与版本准备

功能和修复通过 PR 合入 `main`。PR 及 `main` 推送会运行前端类型检查、单元测试、Vite 构建、视觉 manifest 校验、版本检查器测试、Rust 格式检查、Core Clippy 与测试，并在 macOS/Windows 检查桌面 crate。视觉 manifest 检查读取现有截图和清单，不会在 CI 中重新截图，也不代表所有平台都做过视觉验收。

每个要发布的 PR 都应同步递增版本号。版本号以根 `Cargo.toml` 的 `[workspace.package].version` 为基准，并同步到 `src-tauri/tauri.conf.json`、`frontend/package.json` 和 `frontend/package-lock.json`。例如准备 `0.1.1-rc.1`：

```bash
# 在版本 PR 中将根 Cargo.toml 的 workspace.package.version 改为 0.1.1-rc.1
npm --prefix frontend version 0.1.1-rc.1 --no-git-tag-version
# 同步 src-tauri/tauri.conf.json 的 version 为 0.1.1-rc.1
cargo check --workspace
cargo check --workspace --locked
node tools/check-release-version.mjs v0.1.1-rc.1
```

检查并提交 `Cargo.lock` 中 MauLink 自身 package 的版本变化；确认没有无关依赖升级。`check-release-version.mjs` 也检查严格的 `vX.Y.Z` / `vX.Y.Z-rc.N` SemVer 标签、npm lock 根版本以及正式应用标识 `io.maulink.desktop`。不要为测试构建更改正式标识、应用数据目录或凭据行为。

版本递增 PR 合并到 `main` 后，`release.yml` 会校验版本、运行发布检查、构建三个平台的安装包，并在所有附件上传成功后自动公开 Release。它会在合并提交上创建与版本匹配的 `vX.Y.Z` 或 `vX.Y.Z-rc.N` 标签；无需手动打 Tag 或点击发布。未改变应用版本号的普通合并会跳过发布。重复使用已存在的版本标签会让工作流失败，必须先递增版本。

## 自动构建与发布

`release.yml` 先校验 Tag、版本、前端、视觉清单、Rust 格式和 Core 测试，再分别在原生 Runner 构建：

| 平台 | Rust target | Release 附件 |
| --- | --- | --- |
| macOS Apple Silicon | `aarch64-apple-darwin` | `.dmg` 安装包、`.app.tar.gz` 更新包及其 `.sig` |
| macOS Intel | `x86_64-apple-darwin` | `.dmg` 安装包、`.app.tar.gz` 更新包及其 `.sig` |
| Windows x64 | `x86_64-pc-windows-msvc` | `.msi` 安装包及其 `.sig` |

签名版本发布还要求先启用 GitHub Pages（`Settings → Pages → Source = GitHub Actions`）和 `maulink-updater-release` Environment。该 Environment 必须限制到 `main` 并由负责人配置 required reviewers；其中放置 `TAURI_SIGNING_PRIVATE_KEY` 及可选的 `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`。仓库 Actions Variables 提供 `MAULINK_UPDATER_PUBLIC_KEY`。PR 与 Packaging Smoke 不读取签名私钥。

发布前置检查会验证 Pages 已启用，然后三个原生构建 job 才可读取 Environment 私钥。每个平台必须同时生成对应安装包和 updater 签名；Release job 核对 8 个平台文件并生成 `SHA256SUMS`，创建 Draft 并在上传完成后自动公开。随后独立 job 使用公钥复验三平台签名、检查 GitHub Release 下载地址，按 SemVer 仅更新 Stable 或 Preview 对应的 Pages JSON 索引。索引部署成功前，应用内更新不能取得新版本。版本包含预发布标识（例如 `-rc.8`）时，Release 会标记为 prerelease。Release 和 Tag 创建失败会让工作流失败，不会覆盖已有版本。

Updater `.sig` 用于 Tauri 包完整性和发布者验证，不是 Apple Developer ID / 公证或 Windows Authenticode 签名。Mac 应用仍为 ad-hoc 签名且未公证，Windows MSI 仍未做 Authenticode 可信签名。

macOS 构建使用 `APPLE_SIGNING_IDENTITY='-'` 进行 ad-hoc 签名且**未公证**。Windows MSI 当前**未做 Authenticode 可信签名**。这些产物是供验收的测试包，不应描述成已获得普通用户可信分发能力。PR Smoke 不使用任何签名 Secret；正式 Release 使用 Tauri Updater 密钥，但不需要 Apple 或 Windows 代码签名证书。

## 下载与验收

工作流成功后，从 GitHub Releases 下载附件，并先在下载目录校验哈希。更新设置索引默认部署于 `https://mauriceliang.github.io/MauLink/updates/stable.json` 与 `preview.json`。macOS：

```bash
shasum -a 256 -c SHA256SUMS
```

Windows PowerShell：

```powershell
Get-Content .\SHA256SUMS | ForEach-Object {
  $expected, $name = $_ -split '\s+', 2
  $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $name).Hash.ToLowerInvariant()
  if ($actual -ne $expected) { throw "SHA-256 mismatch: $name" }
}
```

在测试账户安装前，先备份 MauLink 应用数据。使用对应 CPU 架构的安装包，记录操作系统版本、安装和启动结果、警告、关键功能及覆盖安装后的资料留存情况。测试至少覆盖 SSH、SFTP、Terminal、Monitor、原生文件选择和系统凭据读取。macOS 可能因 ad-hoc 签名/隔离属性阻止启动；Windows 可能显示 SmartScreen 警告。**不要要求用户关闭系统安全功能，也不要把未签名或未公证的状态说成已通过。**

macOS 可检查应用主程序架构：

```bash
file /Applications/MauLink.app/Contents/MacOS/maulink
```

在 M1 Pro 或其他 Apple Silicon 设备检查 ARM64 包；Intel 支持结论需在真实 Intel Mac 或可信远程 Intel 环境验收。Windows MSI 必须在真实 x64 Windows 检查安装、卸载、WebView2、凭据和覆盖安装。

## 发布结果与回退

Release 会在自动检查、三平台构建和附件上传成功后公开。macOS 包仍为 ad-hoc 签名且未公证，Windows MSI 未做 Authenticode 可信签名；公开页面的发布说明会明确这些状态。已公开版本出现问题时，递增版本并发布修复版，不要移动或重写已发布 Tag。

自动检查或构建失败时，发布 job 不会运行；附件上传失败时，Release 保持 Draft，不会公开。若 Release 已公开后发现问题，保留既有 Tag 与旧版本附件，通过修复 PR 递增版本并自动发布修复版，不要覆写旧 Tag。回退前备份应用数据，并确认数据库迁移是否允许旧应用读取；不要因降级而删除或重建本地数据库和凭据。真实更新源配置完成前，回退由用户手动安装先前可信版本完成。

## 后续阶段

- **签名与公证**：需要用户办理 Apple Developer ID、公证权限及 Windows Authenticode 签名方案，并将材料仅配置在受保护的 GitHub Secrets / Environment。取得实际设备验证结果后再修改发布流程。不得将证书或私钥放入仓库、日志或聊天。
- **Tauri Updater**：Stable/Preview 偏好、严格 SemVer、Rust 端渠道选择与候选检查、Mock 流程和安装安全门禁已实现。正式发布工作流现可生成三平台签名更新包、复验 `.sig` 并将双渠道索引部署到 GitHub Pages；索引只在公开 Release 资产可下载且签名复验成功后更新。仓库当前 Pages 仍未启用，且尚无 Updater 公钥变量和 Environment 私钥 Secrets，因此真实在线更新仍禁用。降级在下载前一律拦截，直到数据库兼容策略、备份和 Windows MSI/macOS 实机测试完成；MSI overlay 虽显式设置 `allowDowngrades=true`，不能据此宣称降级安装已验证。负责人仍需启用 Pages、配置受保护发布 Environment 并安全保管签名私钥，之后完成首个带配置客户端的手动安装与真机验收。不得生成伪造 `.sig` / `stable.json` / `preview.json`。详见 [UPDATING.md](UPDATING.md)。
- Linux、Windows ARM64、Portable ZIP 和 Universal macOS 包均需另行确认范围和真实设备验收。

## 历史 CI/CD 验收报告（2026-10-12）

以下记录描述旧版 Tag 触发 Draft Release 工作流的验收结果；当前发布规则已改为版本递增合入 `main` 后自动公开 Release。

- 审查日期：2026-10-12
- 审查基线 commit：`bb0ccf8dcdc79a5e57d0f5b2034503ba48af65fc`（`main`）
- 当前分支：`dev/app-updates`（遵循现有 worktree 约定；不新建分支）
- 本次文件：`.github/workflows/ci.yml`、`.github/workflows/release.yml`、`tools/check-release-version.mjs`、`tools/check-release-version.test.mjs`、`frontend/package.json`、`frontend/package-lock.json`、`frontend/tests/geoip-settings.test.ts`、`frontend/tests/server-overview.test.ts`、`RELEASING.md`

干净安装首次发现 lockfile 缺少 DOMPurify 已声明的可选类型包条目；已精确锁定 `@types/trusted-types@2.0.7`，没有更新其他依赖。另将 GeoIP 测试定位到“自动更新频率”控件，并同步两条已过期的网络信息文案断言，保留全部业务检查。

### CI 结果

| 检查 | 本地结果 | GitHub Actions |
| --- | --- | --- |
| 前端类型、单测、构建、视觉清单和版本脚本测试 | PASS：`npm ci`、`type-check`、273 项 Vitest、Vite build、7 项版本脚本测试及视觉清单检查 | NOT RUN：当前分支尚无 PR；待负责人创建 PR |
| Rust Core fmt、Clippy、测试 | PASS：`cargo fmt`、Clippy `-D warnings`、Core 与合约测试共 111 项通过；11 项原生凭据 / OpenSSH fixture 测试按仓库标记忽略 | NOT RUN：当前分支尚无 PR；待负责人创建 PR |
| macOS Desktop Check | PASS：本机 `cargo check -p maulink-desktop --locked` | NOT RUN：当前分支尚无 PR；待负责人创建 PR |
| Windows Desktop Check | 本机没有 Windows Runner，未运行 | NOT RUN：当前分支尚无 PR；待负责人创建 PR |

本地其他验证：两个工作流 YAML 均可解析，所有 Bash `run` 脚本通过 `bash -n`；Web visual Harness 的 `transfer / dark / zh-CN` 页面成功打开。视觉 manifest 校验报告 9 个 manifest、82 张截图，独立重复结果 byte-identical。

本机 ARM64 安装包验证：PASS — `target/aarch64-apple-darwin/release/bundle/dmg/MauLink_0.1.0_aarch64.dmg`（12.60 MiB），`hdiutil verify` 返回 VALID；只读挂载后确认内含 arm64 Mach-O，`codesign --verify --deep --strict` 通过。使用 `CI=true` 运行以匹配 GitHub Actions，并跳过本机 Finder AppleScript；包为 ad-hoc 签名、未公证。这是本机验收产物，不是 GitHub Release 附件。

### 发布结果

- `v0.1.1-rc.1`：负责人已授权并推送。分支可达性和版本一致性校验 PASS；前端单测 `files.test.ts` 有 1 项在 GitHub Runner 上超过 5 秒而超时，验证 job 失败，三个平台构建均跳过，未创建 Draft Release。
- `v0.1.1-rc.2`：版本、分支可达性、前端检查和版本脚本检查 PASS；Linux Core 测试因 `credentials.rs` 中不支持平台分支仍引用未定义的 `store` 而编译失败（E0425），三个平台构建跳过，未创建 Draft Release。既有 `rc.2` Tag 保持不变。
- `v0.1.1-rc.3`：发布版本和分支检查、前端检查、Linux Core fmt/test、macOS ARM64 与 Intel DMG 全部 PASS，确认凭据存储条件编译修复生效。Windows checkout 被 `docs/refactor/screenshots/color-v1-review/` 下超长清单文件名阻断，未进入 MSI 构建；未创建 Draft Release。既有 `rc.3` Tag 保持不变。
- `v0.1.1-rc.4`：Windows checkout 的 `core.longpaths=true` 修复生效；发布校验、前端、Linux Core、macOS ARM64 与 Intel DMG 全部 PASS。Windows MSI 编译失败：`native_identity.rs` 从错误模块导入 `IAsyncOperation`（E0432），并把 Tauri 的 `windows 0.61.3` HWND 传给直接依赖的 `windows 0.62.2` API（E0308）；未创建 Draft Release。既有 `rc.4` Tag 保持不变。
- `v0.1.1-rc.5`：发布校验、前端、Linux Core 和 macOS ARM64 DMG PASS；Windows checkout 与 `windows` API 类型修复 PASS。Windows MSI 编译在把 `IAsyncOperation` 直接传给 `block_on` 时失败（E0277：该类型通过 `IntoFuture` 工作），Intel DMG 当时仍在构建；未创建 Draft Release。既有 `rc.5` Tag 保持不变。
- `v0.1.1-rc.6`：发布校验、前端、Linux Core 和 macOS Intel DMG PASS；Windows Rust 可执行文件编译 PASS，但 MSI bundler 拒绝 `0.1.1-rc.6` 预发布版本格式，未创建 Draft Release。macOS ARM64 首次 `bundle_dmg.sh` 失败；同 SHA 的 job 重跑成功。
- macOS ARM64 DMG：本机 `0.1.1-rc.2` PASS — `target/aarch64-apple-darwin/release/bundle/dmg/MauLink_0.1.1-rc.2_aarch64.dmg`（12.60 MiB），`hdiutil verify` 返回 VALID；只读挂载确认 arm64 Mach-O、版本 `0.1.1-rc.2`、标识 `io.maulink.desktop`，`codesign --verify --deep --strict` 通过。SHA-256：`9ddcb1a6cd5dd7f55acfedd783af6f8e96cc67478e92be0b52959a3c88921a9c`。首次沙箱构建在 DMG bundler 失败，授权重试后通过；产物为 ad-hoc 签名、未公证。GitHub Actions `rc.3`、`rc.4`、`rc.5` 构建 PASS。`rc.6` 同 SHA 重跑 PASS，产物 `maulink-macos-arm64`（13,164,293 bytes）；首次失败未稳定复现，根因未确认。
- macOS Intel DMG：`v0.1.1-rc.3`、`rc.4`、`rc.6` GitHub Actions 构建 PASS；`rc.5` Intel job 尚未完成。
- Windows x64 MSI：`rc.3` 因 checkout 路径过长失败；`rc.4`、`rc.5` 分别遇到 WinRT API 类型编译错误；`rc.6` Rust 可执行文件编译 PASS，MSI bundler 因 SemVer prerelease 格式失败。实现了仅用于 MSI `ProductVersion` 的映射和实际 MSI 属性检查；需由 PR smoke job 在 Windows Runner 验证真实 MSI。
- SHA256SUMS：未生成；随三平台构建通过后由工作流生成。
- Draft Release URL：无。
- 公开发行状态：未公开；工作流只创建 Draft，发布需负责人审查确认。

### 安全、设备与待办

- Actions 权限：全局 `contents: read`；仅 Draft 发布 job 使用 `contents: write`。
- 用户 Secrets：本次未使用或写入，未出现在日志。
- 正式 Tauri identifier：`io.maulink.desktop`，检查器强制校验；未修改。
- 测试配置：正式构建仅使用正式 `tauri.conf.json`；未采用 harness 或 frontend-v2 配置。
- 签名状态：本机 ARM64 DMG 为 ad-hoc / 未公证，签名校验通过；Windows unsigned；真实设备安装尚未验证。
- Tauri Updater：未启用。
- `npm ci` 审计提示当前依赖树有 2 项 advisories（1 low、1 high）；本次没有升级依赖，需单独评估和修复。
- Apple Silicon Mac、Intel Mac、Windows x64 真机、覆盖安装保留数据：均 NOT RUN。
- GitHub PR 运行：由负责人创建 PR 后验证并按需配置 required checks。
- 测试 Tag：`v0.1.1-rc.1` 至 `v0.1.1-rc.6` 均保留原值；本次未创建新 Tag。后续版本测试须在实现修复并经 PR 验证后另行决定，不覆写既有 Tag。
- 公开稳定发行前：负责人确认许可证和第三方依赖许可；仓库目前缺少许可证文本。
- 签名、公证、Windows 可信签名和 Updater：后续阶段，需用户提供相应账户配置或密钥管理方案。

### RC6 Windows MSI 与打包烟测修复（2026-10-12）

- 原始故障：`v0.1.1-rc.6` Windows job 已成功编译 `maulink.exe`，MSI bundler 随后报错 `optional pre-release identifier in app version must be numeric-only and cannot be greater than 65535 for msi target`。macOS ARM64 原 job 的 `bundle_dmg.sh` 失败；同一提交、同一 runner 的重跑成功并上传 DMG，因此未改 macOS 打包路径，失败原因仍未知。
- MSI 版本规则：只接受 `MAJOR.MINOR.PATCH` 与 `MAJOR.MINOR.PATCH-rc.N`。映射 `ProductVersion = MAJOR.MINOR.(PATCH * 1000 + ordinal)`；RC ordinal 为 `1..998`，稳定版为 `999`。major/minor 上限为 `255`，patch 上限为 `64`。例如 `0.1.1-rc.6 -> 0.1.1006`，`0.1.1 -> 0.1.1999`，确保 RC、稳定版和下一 patch RC 的 Windows Installer 三段版本递增。超出范围或其他 prerelease 会在打包前失败。Tauri 正式版本、bundle identifier 与 UpgradeCode 未修改。
- `tools/msi-version.mjs` 提供纯映射函数和边界测试；`tools/write-msi-config.mjs` 校验 tag 与应用版本一致后生成被 Git 忽略的 Windows 配置覆盖文件。release workflow 只在 Windows 构建传入该覆盖配置，并通过 Windows Installer COM 读取最终 MSI 的 `ProductVersion` 核验。PR CI 增加映射测试。
- `.github/workflows/packaging-smoke.yml` 为 PR 和手动触发的三平台打包检查，权限为 `contents: read`，只上传保留 7 天的测试安装包；macOS job 另外收集失败日志、Runner 信息与生成的 `bundle_dmg.sh`。该工作流不创建 Tag、Release 或使用签名 Secret。PR 尚未由负责人创建，GitHub 三平台 smoke jobs 与 Windows 实际 MSI 检查仍待运行。
- RC6 重跑记录：[GitHub Actions run 38205431669](https://github.com/MauriceLiang/MauLink/actions/runs/38205431669)：macOS ARM64 retry 与 Intel job 成功；Windows MSI job 因原 prerelease 版本格式失败；没有 Draft Release。
- 本地验证：MSI 映射测试 22 项、版本检查测试 8 项通过；`cargo fmt --all -- --check`、Core Clippy、Core 与合约测试通过（111 项通过，10 项按仓库标记忽略）；前端类型检查、273 项 Vitest、Vite build 与 82 张截图的视觉清单校验通过。3 个 loopback 单测首次被沙盒拒绝，获准后重跑通过。Web visual Harness 的 transfer / dark / zh-CN 页面已在当前工作树打开。
- 当前 tag `v0.1.1-rc.6` 没有被移动或重写，本次没有创建新 Tag、Release 或 PR。`cargo tauri inspect wix-upgrade-code` 得到 `9d777ff5-45a9-5b41-aa62-fcab3883e989`；尚无既有成功 MSI 可供核对，因此不能据此确认覆盖安装兼容性。
