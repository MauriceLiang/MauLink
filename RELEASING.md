# MauLink 构建与发布

本文说明 MauLink 的 PR 检查、跨平台安装包构建和 GitHub Draft Release 流程。首期目标是 macOS Apple Silicon、macOS Intel 与 Windows x64。Linux、Windows ARM64、可信代码签名、公证和应用内更新均不在本期支持范围。

## 首次启用

1. 在 GitHub 仓库确认 `Settings → Actions → General` 允许本仓库工作流运行，并查看 Actions 使用量、缓存和 14 天构建产物保留所产生的费用或配额影响。
2. 将 `CI` 工作流的以下状态检查设置为 `main` 的 required checks：
   - `Frontend / typecheck / unit / build`
   - `Rust Core / fmt / clippy / test`
   - `Desktop check / macos-15`
   - `Desktop check / windows-2025`
3. 首次 PR 成功运行后再设置 required checks；状态检查名称以 GitHub 展示的实际 job 名称为准。
4. PR 工作流只需要 `contents: read`。创建 Draft Release 的唯一 job 单独获得 `contents: write`。不要使用 `pull_request_target` 执行 PR 代码，不要把发布 Token、证书或更新私钥传给 PR job。

工作流使用 GitHub-hosted `ubuntu-24.04`、`macos-15`、`macos-15-intel` 和 `windows-2025`。Action 使用 `actions/checkout@v7`、`actions/setup-node@v7`、`actions/upload-artifact@v7` 和 `actions/download-artifact@v8`；升级这些版本时先核对官方兼容说明及 Runner 要求。

## PR 与版本准备

功能和修复通过 PR 合入 `main`。PR 及 `main` 推送会运行前端类型检查、单元测试、Vite 构建、视觉 manifest 校验、版本检查器测试、Rust 格式检查、Core Clippy 与测试，并在 macOS/Windows 检查桌面 crate。视觉 manifest 检查读取现有截图和清单，不会在 CI 中重新截图，也不代表所有平台都做过视觉验收。

发布版本使用独立版本 PR。版本号以根 `Cargo.toml` 的 `[workspace.package].version` 为基准，并同步到 `src-tauri/tauri.conf.json`、`frontend/package.json` 和 `frontend/package-lock.json`。例如准备 `0.1.1-rc.1`：

```bash
# 在版本 PR 中将根 Cargo.toml 的 workspace.package.version 改为 0.1.1-rc.1
npm --prefix frontend version 0.1.1-rc.1 --no-git-tag-version
# 同步 src-tauri/tauri.conf.json 的 version 为 0.1.1-rc.1
cargo check --workspace
cargo check --workspace --locked
node tools/check-release-version.mjs v0.1.1-rc.1
```

检查并提交 `Cargo.lock` 中 MauLink 自身 package 的版本变化；确认没有无关依赖升级。`check-release-version.mjs` 也检查严格的 `vX.Y.Z` / `vX.Y.Z-rc.N` SemVer 标签、npm lock 根版本以及正式应用标识 `io.maulink.desktop`。不要为测试构建更改正式标识、应用数据目录或凭据行为。

版本 PR 合并且 CI 成功后，由仓库负责人检查版本和变更记录，再明确批准并推送版本 Tag。RC 示例：

```bash
git switch main
git pull --ff-only origin main
git tag v0.1.1-rc.1
git push origin v0.1.1-rc.1
```

正式版本按相同流程使用 `vX.Y.Z`。**不要移动或重写已经公开的 Tag。**本实施不创建版本 Tag；`release.yml` 仅由已存在的 `v*` Tag 触发，并会阻止不在 `main` 历史中的提交。

## 自动构建与 Draft Release

`release.yml` 先校验 Tag、版本、前端、视觉清单、Rust 格式和 Core 测试，再分别在原生 Runner 构建：

| 平台 | Rust target | Draft 附件 |
| --- | --- | --- |
| macOS Apple Silicon | `aarch64-apple-darwin` | `MauLink_<version>_aarch64-apple-darwin.dmg` |
| macOS Intel | `x86_64-apple-darwin` | `MauLink_<version>_x86_64-apple-darwin.dmg` |
| Windows x64 | `x86_64-pc-windows-msvc` | `MauLink_<version>_x86_64-pc-windows-msvc.msi` |

每个构建必须恰好产生一个对应安装包。只有三个 job 全部成功后，发布 job 才收齐并再次核对这三个文件、生成 `SHA256SUMS`，然后用 `gh release create --draft --verify-tag` 创建单一 Draft Release。RC Tag 会设置为 prerelease。现有同名 Release 会导致创建失败；工作流不会删除或覆盖它。

macOS 构建使用 `APPLE_SIGNING_IDENTITY='-'` 进行 ad-hoc 签名且**未公证**。Windows MSI 当前**未做 Authenticode 可信签名**。这些产物是供验收的测试包，不应描述成已获得普通用户可信分发能力。PR 不使用签名 Secret；此工作流也不需要 Apple、Windows 或 Updater 密钥。

## 下载与验收

只从 Draft Release 下载附件，并先在下载目录校验哈希。macOS：

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

## Draft 审核、发布与回退

所有平台产物、SHA256SUMS 和发布说明均通过检查后，负责人在 GitHub Releases 页面打开 Draft，核对 Tag、三个架构附件、哈希和签名状态，补充该版本实际变更说明，并在真实设备验收后决定是否点击 **Publish release**。发布公开版本还需负责人确认许可证与第三方依赖许可；当前仓库尚未提供许可证文本，需在首次公开稳定发行前处理。

发现问题时保持 Draft，不发布。若已公开，保留既有 Tag 与旧版本附件，撤回或标记有问题的 Release，并通过修复 PR 与新版本 Tag 发布修复；不要覆写旧 Tag。回退前备份应用数据，并确认数据库迁移是否允许旧应用读取；不要因降级而删除或重建本地数据库和凭据。Updater 尚未启用，回退由用户手动安装先前可信版本完成。

## 后续阶段

- **签名与公证**：需要用户办理 Apple Developer ID、公证权限及 Windows Authenticode 签名方案，并将材料仅配置在受保护的 GitHub Secrets / Environment。取得实际设备验证结果后再修改发布流程。不得将证书或私钥放入仓库、日志或聊天。
- **Tauri Updater**：单独设计和验收 runtime、签名密钥、公钥、权限、平台归档、更新 metadata 与用户交互。用户提供并安全保管 Updater 私钥前，不开启自动检查或安装更新，也不生成伪造 `.sig` / `latest.json`。
- Linux、Windows ARM64、Portable ZIP 和 Universal macOS 包均需另行确认范围和真实设备验收。

## MauLink CI/CD 验收报告

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

- Tag：未创建；测试 Tag 需要负责人明确批准。
- Validate job：NOT RUN；需负责人批准 Tag 后触发。
- macOS ARM64 DMG：本机 PASS，见上方文件名和大小；GitHub Actions 附件 NOT RUN，待负责人批准测试 Tag 后记录。
- macOS Intel DMG：NOT RUN；附件名、大小待真实 Runner 构建后记录。
- Windows x64 MSI：NOT RUN；附件名、大小待真实 Runner 构建后记录。
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
- 首次测试 Tag：由负责人确认版本（例如 `v0.1.1-rc.1`）并明确批准后再推送。
- 公开稳定发行前：负责人确认许可证和第三方依赖许可；仓库目前缺少许可证文本。
- 签名、公证、Windows 可信签名和 Updater：后续阶段，需用户提供相应账户配置或密钥管理方案。
