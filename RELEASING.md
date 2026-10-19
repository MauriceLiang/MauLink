# MauLink 构建与发布

本次改造移除了仓库中的全部 GitHub Actions 工作流。该改造合并后，推送分支、创建 PR 或推送 Tag 都不会自动运行检查、构建、上传产物、创建 Release 或部署应用内更新索引。维护者负责在对应平台手动编译，并在 GitHub Releases 手动上传安装包。

应用内更新界面与本地交互已接入，但真实更新地址和受信公钥尚未配置，在线更新仍未启用。手动上传 GitHub Release 不会自动发布 Stable/Preview 索引，也不会使应用内更新生效；详见 [UPDATING.md](UPDATING.md)。不要伪造 `.sig`、`stable.json` 或 `preview.json`。

## 版本准备

以根 `Cargo.toml` 的 `[workspace.package].version` 为版本基准，并同步更新 `src-tauri/tauri.conf.json`。运行以下命令更新前端 package 和 lockfile：

```bash
npm --prefix frontend version 0.1.1-rc.34 --no-git-tag-version
cargo check --workspace
cargo check --workspace --locked
node tools/check-release-version.mjs v0.1.1-rc.34
```

先把根 Cargo 和 Tauri 配置版本改为同一新版本，再运行检查。确认 `Cargo.lock` 只包含 MauLink 自身版本变化，没有无关依赖升级。版本检查器会核对 SemVer、npm lock 根版本和应用标识 `io.maulink.desktop`。不要为测试构建更改正式应用标识、应用数据目录或凭据行为。

## 手动构建安装包

在对应平台的原生环境中安装前端依赖和 Rust target，再运行对应命令。Tauri 的 `beforeBuildCommand` 会构建前端：

```bash
npm --prefix frontend ci

# macOS Apple Silicon
CI=true cargo tauri build --target aarch64-apple-darwin --bundles dmg --ci -- --locked

# macOS Intel（需安装 x86_64-apple-darwin target）
CI=true cargo tauri build --target x86_64-apple-darwin --bundles dmg --ci -- --locked

# Windows x64（在 Windows x64 原生环境执行）
node tools/write-msi-config.mjs
cargo tauri build --config src-tauri/tauri.release-windows.conf.json --target x86_64-pc-windows-msvc --bundles msi --ci -- --locked
```

安装包输出在 `target/<Rust target>/release/bundle/{dmg,msi}/`。Windows 临时 MSI 配置用于把 RC 版本映射为 Windows Installer 支持的 `ProductVersion`；它由脚本生成，不要提交生成文件。构建后核对应用版本、CPU 架构和安装包格式。macOS 包使用 ad-hoc 签名且未公证；Windows MSI 未做 Authenticode 可信签名。

## 手动上传与验收

在 GitHub Releases 页面手动创建与应用版本一致的 Tag 和 Release，RC 版本勾选 prerelease，上传对应平台安装包及 `SHA256SUMS`。macOS 可用 `shasum -a 256 <安装包...> > SHA256SUMS` 生成校验清单；下载后使用 `shasum -a 256 -c SHA256SUMS` 校验。记录版本、操作系统、CPU 架构、构建结果、签名状态和已完成的安装验收；不要把未经验证的平台标为已通过。

在测试账户安装前备份 MauLink 应用数据。用对应 CPU 架构的设备检查安装、首次启动、SSH、SFTP、Terminal、Monitor、原生文件选择、系统凭据读取及覆盖安装后的数据留存。macOS 可能因 ad-hoc 签名或隔离属性显示安全提示；Windows 可能显示 SmartScreen 警告。**不要要求用户关闭系统安全功能，也不要把未签名或未公证的状态描述成已通过。**

若已公开版本存在问题，保留原 Tag 和附件，通过新版本发布修复版，不要移动或重写已发布 Tag。回退前备份应用数据并确认数据库兼容性；真实更新源配置前，回退由用户手动安装先前版本完成。

## 后续阶段

- **签名与公证**：如需普通用户可信分发，需配置 Apple Developer ID、公证权限和 Windows Authenticode 签名，并在真实设备上验收。不得将证书或私钥放入仓库、日志或聊天。
- **Tauri Updater**：Stable/Preview 偏好、严格 SemVer、Rust 端渠道选择与安装安全门禁已实现，但在线更新仍未启用。启用前需配置受信签名密钥、更新索引托管和人工发布流程，并完成真机验收；普通 GitHub Release 附件不能被视为已验证的应用内更新。
- Linux、Windows ARM64、Portable ZIP 和 Universal macOS 包需另行确认范围并在真实设备验收。

## 历史 CI/CD 验收报告（2026-10-12）

以下记录是 GitHub Actions 尚在启用时的历史验收报告，不代表当前仍有自动化工作流。

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
