**[简体中文](./README.md)** | English

<div align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="./frontend/src/assets/app-icon-dark.png">
    <source media="(prefers-color-scheme: light)" srcset="./frontend/src/assets/app-icon-light.png">
    <img src="./frontend/src/assets/app-icon-light.png" alt="MauLink logo" width="112" height="112">
  </picture>
  <h1>MauLink</h1>
  <p><strong>Connect to your servers. Stay focused.</strong></p>
  <p>Bring SSH terminals, remote files, and host monitoring into one local desktop workspace.</p>
  <p>
    <img src="https://img.shields.io/badge/version-0.1.0-3b82f6?style=flat-square" alt="Version 0.1.0">
    <img src="https://img.shields.io/badge/status-development-64748b?style=flat-square" alt="Development and internal testing">
    <img src="https://img.shields.io/badge/Tauri-2-24c8db?style=flat-square" alt="Tauri 2">
    <img src="https://img.shields.io/badge/Rust-2024-dc7844?style=flat-square" alt="Rust 2024 edition">
    <img src="https://img.shields.io/badge/Vue-3-42b883?style=flat-square" alt="Vue 3">
  </p>
  <p>
    <a href="#ui-preview">UI preview</a> ·
    <a href="#core-capabilities">Core capabilities</a> ·
    <a href="#quick-start">Quick start</a> ·
    <a href="#architecture-and-engineering">Architecture and engineering</a> ·
    <a href="#verification-and-release-status">Verification and release status</a>
  </p>
</div>

---

MauLink is for developers who frequently connect to remote hosts. Organize servers into groups, then switch among terminals, files, and monitoring in one workspace. The app connects directly from your device to remote hosts; it does not require a MauLink cloud service or a self-hosted relay.

**Local-first** · **Explicit host trust** · **Unified connection workspace** · **Light / dark themes · Chinese and English**

MauLink is currently in development and internal testing, targeting macOS and Windows. A native macOS arm64 test build has been verified. Windows device acceptance, release signing, and macOS notarization are still pending.

## UI preview

<table>
  <tr>
    <th>Light theme · a clear server overview</th>
    <th>Dark theme · a consistent workspace</th>
  </tr>
  <tr>
    <td><img src="./docs/refactor/screenshots/color-v1-review/servers-light-zh-CN-1440x920.jpg" alt="MauLink Blue server home in light theme" width="480"></td>
    <td><img src="./docs/refactor/screenshots/color-v1-review/servers-dark-zh-CN-1440x920.jpg" alt="MauLink Blue server home in dark theme" width="480"></td>
  </tr>
</table>

<details>
<summary><strong>Show the Terminal and Quick Monitor workspace</strong></summary>

![Terminal and Quick Monitor workspace](./docs/refactor/screenshots/color-v1-review/terminal-light-zh-CN-1440x920.jpg)

</details>

<details>
<summary><strong>Show the unified select control and top-level feedback</strong></summary>

![Dark dialog and theme selector](./docs/refactor/screenshots/color-v1-review/palette-dark-zh-CN-1440x920.jpg)

![Success notification centered at the top](./docs/refactor/screenshots/color-v1-review/toast-light-zh-CN-1440x920.jpg)

</details>

The screenshots show the current MauLink Blue Vue UI. Server lists, terminal output, and monitoring metrics use isolated Browser Harness test data. See the [visual regression guide](./frontend/visual/README.md) for fixed scenarios and screenshot checks.

## Core capabilities

| Workflow | What MauLink provides |
| --- | --- |
| **Manage servers** | Create, edit, delete, and search servers and groups; revision checks prevent silent overwrites |
| **Connect over SSH** | Password or private-key authentication, one-hop Jump Hosts, SOCKS5 / HTTP CONNECT proxies, connection tests, cancellation, timeouts, and keepalive |
| **Verify host identity** | Confirm a Host Key on first connection; changed trusted keys are rejected by default and can be updated after verification |
| **Use terminals** | Multiple independent PTYs, adaptive sizing, focus mode, font and cursor settings; xterm.js is bundled with the app |
| **Manage remote files** | SFTP pagination, metadata, directory creation, rename, delete, and single-file upload / download; view and edit plain-text files up to 2 MiB, with Markdown preview and save-conflict checks |
| **Monitor hosts** | Shared snapshots for Quick / Full Monitor; CPU, memory, root filesystem, network, load, uptime, and data-quality status |
| **Set up your workspace** | System / light / dark themes, Simplified Chinese / English, command palette, six notification positions, terminal preferences, and disconnect confirmation |
| **Keep data on your device** | SQLite stores configuration, Host Keys, and settings; for each server, passwords and private-key passphrases can be stored in the system credential store or encrypted with AES-GCM in the local database |

Jump Hosts use the destination server's authentication method and credentials. Specify a different username with <code>user@host</code>. Proxies currently support unauthenticated SOCKS5 and HTTP CONNECT. Remote text viewing and editing is limited to plain-text files up to 2 MiB. Binary preview / editing, Docker management, database clients, process lists, and Disk I/O monitoring are not currently supported.

### Icons and appearance

The interface uses **MauLink Blue + White / Slate Neutral**: Primary is <code>#3B82F6</code> in light theme and <code>#60A5FA</code> in dark theme. Primary buttons, focus, and selected states share semantic tokens. Pages, sidebars, and cards use neutral colors; success, warning, and destructive actions use their respective status colors.

Reka UI and the existing <code>Base*</code> components provide consistent selects, menus, dialogs, and switches, with keyboard navigation, focus restoration, and viewport-aware positioning. Notifications appear at the top right by default; settings offer six preset positions. They use a neutral two-line card, with status colors reserved for the icon, and dismiss automatically based on their content. Destructive actions that need confirmation still use a confirmation dialog. Animations are lightweight and support reduced-motion preferences.

The brand logo follows the light or dark theme. The separate “App icon style” setting selects a light or dark icon, updates the running app icon after saving, and restores it after restart. The macOS Dock uses the selected style, while **Finder always uses the light rounded install icon**. The icons have real transparent rounded corners. See [app_icon.rs](./src-tauri/src/app_icon.rs) for runtime switching and [generate-app-icons.mjs](./tools/generate-app-icons.mjs) for asset generation.

## Quick start

### Prerequisites

| Environment | Requirement |
| --- | --- |
| Rust | stable, minimum <code>1.89</code>; includes <code>rustfmt</code> and <code>clippy</code> |
| Node.js | <code>^20.19.0 || &gt;=22.12.0</code>, with npm |
| Tauri CLI | <code>2.12.0</code> |
| macOS | Xcode or Xcode Command Line Tools |
| Windows | MSVC C++ Build Tools and Microsoft Edge WebView2 Runtime |

See the [official Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/). Run the following commands from the repository root.

### Run the desktop development app

~~~bash
cargo install tauri-cli --version 2.12.0 --locked
npm --prefix frontend ci
cargo tauri dev
~~~

Tauri starts Vite and loads the Vue frontend with HMR. Stop any standalone Vite server first to avoid a port conflict on <code>1420</code>. The app uses the production identifier <code>io.maulink.desktop</code> and the normal local data directory.

### Debug only the frontend

~~~bash
npm --prefix frontend run dev
~~~

Open <code>http://127.0.0.1:1420/?harness=visual&amp;page=servers&amp;theme=light&amp;locale=zh-CN</code> to use the DEV-only Typed Mock IPC and test UI with data isolated from production records. <code>http://127.0.0.1:1420/?harness=interaction</code> provides an independent preview of base controls, keyboard interaction, overlays, and notifications. The regular Browser entry point requires Native IPC. The Harness is excluded from release builds. See the [frontend development guide](./frontend/README.md) and [visual regression guide](./frontend/visual/README.md) for more scenarios.

### Build the app

~~~bash
# Build the default bundle for the current platform
cargo tauri build -- --locked

# macOS: build only the .app
cargo tauri build --bundles app --ci -- --locked
~~~

Artifacts are written to <code>target/release/bundle/</code>. The macOS app is at <code>target/release/bundle/macos/MauLink.app</code>. The current macOS acceptance build uses a local ad-hoc signature and is not notarized; it is a development test build. See [Verification and release status](#verification-and-release-status) for platform limitations and acceptance dates.

<details>
<summary><strong>Internal macOS packaging and signature checks</strong></summary>

Sign and verify a local test build:

~~~bash
codesign --force --sign - target/release/bundle/macos/MauLink.app
codesign --verify --deep --strict --verbose=2 target/release/bundle/macos/MauLink.app
~~~

Use <code>ditto</code> to preserve bundle metadata in an internal distribution ZIP:

~~~bash
ditto -c -k --sequesterRsrc --keepParent \
  target/release/bundle/macos/MauLink.app \
  target/release/bundle/macos/MauLink_0.1.0_aarch64.zip
~~~

An ad-hoc signature is not a substitute for Developer ID signing and notarization. Historical ZIP files do not represent the latest build; regenerate them as needed.

</details>

## Architecture and engineering

MauLink keeps the boundary between the WebView and system capabilities in the Tauri adapter. Business rules, data access, and SSH subsystems live in the independently testable Rust library <code>maulink-core</code>.

~~~mermaid
flowchart LR
    UI[Local WebView<br/>Vue 3 · TypeScript · Vite · xterm.js]
    Tauri[Tauri 2 adapter<br/>Commands · Channels · file picker · window events]
    Core[maulink-core<br/>business rules · state machines · async resource lifecycle]
    DB[(SQLite<br/>bounded single worker · migrations)]
    Keychain[[macOS Keychain<br/>Windows Credential Store]]
    Remote[Remote SSH server]

    UI -->|Versioned IPC request| Tauri
    Tauri --> Core
    Core <--> DB
    Core <--> Keychain
    Core <-->|russh / SFTP / PTY / Monitor exec| Remote
    Tauri -->|Native file selection and short-lived authorization token| UI
~~~

### Layer responsibilities

| Layer | Path | Responsibility |
| --- | --- | --- |
| Frontend | <code>frontend/</code> | Pages and interactions, IPC calls, terminal-output parsing, and confirmations; no direct access to arbitrary local files or the database |
| Tauri adapter | <code>src-tauri/src/</code> | Register commands, validate request envelopes, adapt Tauri Channels, assemble services, and handle app startup and shutdown |
| Business core | <code>crates/maulink-core/src/</code> | Business rules for profiles, credentials, Host Keys, connections, Terminal, SFTP, Monitor, settings, and storage |
| Type contracts | <code>crates/maulink-core/src/contracts/</code>, <code>contracts/v1/</code> | Rust DTOs, stable IPC payloads, and TypeScript types exported from Rust |
| Local persistence | <code>crates/maulink-core/migrations/</code> | SQLite schema and versioned database migrations |

The production frontend uses Vue 3, exactly pinned TypeScript 5.9.3, and Vite. Reka UI handles complex control interactions; project CSS and semantic tokens define the appearance; modular <code>@tauri-apps/api</code> calls the existing Typed IPC. xterm.js, its fit add-on, and icons are bundled locally, so the app does not need a CDN at runtime. The DEV Harness is for development acceptance only and is excluded from release builds.

<details>
<summary><strong>Versioned IPC contracts and flow control</strong></summary>

Business commands use a versioned request envelope. The read-only app information interface <code>app_get_info</code> retains its signature without an envelope:

~~~json
{
  "apiVersion": 1,
  "requestId": "uuid",
  "payload": {}
}
~~~

<code>requestId</code> associates structured errors with the caller. Errors contain a stable <code>code</code>, a localized <code>messageKey</code>, retryability, a phase, and optional parameters; the UI should not depend on low-level Rust error text. Large integer sequence numbers use decimal strings to avoid JavaScript <code>Number</code> precision loss. Long-lived streams use Tauri Channels. Terminal output uses ACKs to report consumption progress instead of treating “message sent” as “processed by the frontend.”

TypeScript DTOs are generated from Rust types by <code>ts-rs</code> and stored in <code>contracts/v1/</code>. When shared payloads or serialization semantics change, update the Rust DTOs, generated types, and contract tests together.

</details>

### Security and local data

- **Choose a credential storage method per server.** Passwords and private-key passphrases can be stored in macOS Keychain / Windows Credential Manager or encrypted with AES-GCM in local SQLite. Credentials are not written in plaintext to the database or ordinary configuration files. A storage-method change migrates saved credentials when the server configuration is saved; if the system credential service is unavailable, the app does not silently fall back to plaintext files.
- **Credential updates are recoverable.** The database records states such as <code>pending_write</code>, <code>pending_delete</code>, and <code>retained</code>, so failures between SQLite and the system credential store can be identified and cleaned up on a later launch.
- **Changed Host Keys are rejected by default.** Trust is bound to the normalized host and port. Unknown keys require a user decision, and changed keys are never accepted automatically.
- **Native file pickers authorize local files.** The frontend receives a short-lived, purpose-bound token rather than an arbitrary local path. Tokens expire after 10 minutes by default and the registry holds up to 64 entries. Upload and download tokens are consumed after use; a private-key selection token can be used for its corresponding connection until it expires.
- **File management uses the SFTP subsystem.** Remote paths follow POSIX rules; file operations do not build shell commands. Uploads and downloads use bounded chunks and publish through a temporary target.
- **Monitor does not accept frontend commands.** Collection uses fixed backend scripts with time and output limits; text returned by a server is never run as an executable instruction.
- **Tauri commands use explicit capabilities.** The main window can call only the commands the product needs. The app loads a locally bundled page and configures a Content Security Policy.
- **Shutdown and cancellation have resource boundaries.** A connection owns its Terminal, SFTP, transfer, and Monitor child resources. Disconnect and app shutdown cancel and close them in lifecycle order.

These measures reduce sensitive-data exposure and uncontrolled resource risks. They do not guarantee that every in-memory copy in the WebView, operating system, SSH server, or third-party libraries can be completely erased.

### Local data location

The app uses Tauri to locate the current operating system's app data directory, creates <code>maulink.sqlite3</code> there, and runs database migrations. The exact path depends on the operating system and user account. On Unix systems, the app sets the directory permissions to <code>0700</code>. Server configuration, settings, Host Keys, and other app data stay on the device; remote Terminal content is not synced to the cloud.

<details>
<summary><strong>Technology stack and project structure</strong></summary>

### Technology stack

| Technology | Purpose |
| --- | --- |
| Rust 2024 edition, minimum Rust <code>1.89</code> | Desktop backend and business core |
| Tauri <code>2.11.6</code> | Desktop window, IPC, system integration, and app packaging |
| Tokio | Async network tasks, cancellation, and resource lifecycle |
| <code>russh 0.63.3</code> | SSH client, authentication, PTY, and exec channel |
| <code>russh-sftp 3.0.0</code> | SFTP directory operations and file transfer |
| <code>rusqlite 0.38</code>, bundled SQLite | Local database, transactions, migrations, and backups |
| <code>keyring-core</code> and platform-native implementations | System credential-store abstraction |
| Vue 3 / TypeScript 5.9.3 / Vite | Desktop WebView and build |
| xterm.js <code>5.5.0</code>, addon-fit <code>0.10.0</code> | Locally bundled terminal rendering and layout |

<code>Cargo.lock</code> pins Rust dependency resolution. Frontend dependency versions are pinned by <code>frontend/package.json</code> and <code>frontend/package-lock.json</code>.

### Project structure

~~~text
.
├── Cargo.toml                     # Rust workspace and shared dependency versions
├── Cargo.lock                     # Rust dependency lockfile
├── rust-toolchain.toml            # stable, rustfmt, clippy
├── contracts/v1/                  # Generated TypeScript IPC DTOs
├── crates/maulink-core/
│   ├── migrations/                # SQLite schema migrations
│   ├── src/
│   │   ├── contracts/             # IPC data structures and serialization
│   │   ├── connections.rs         # Connection registry and lifecycle
│   │   ├── credentials.rs         # Credential references, recovery, and system store
│   │   ├── host_keys.rs           # Host Key validation and trust records
│   │   ├── local_files.rs         # Local-file tokens
│   │   ├── monitor.rs             # Metrics collection, quality, and history
│   │   ├── profiles.rs            # Servers and groups
│   │   ├── settings.rs            # Typed app settings
│   │   ├── ssh.rs                 # SSH connection and authentication
│   │   ├── sftp.rs                # SFTP and transfer management
│   │   ├── terminal.rs            # PTY and Terminal flow control
│   │   └── storage.rs             # SQLite worker and migrations
│   └── tests/                     # Serialization and OpenSSH integration tests
├── frontend/
│   ├── index.html
│   ├── package.json / package-lock.json
│   ├── src/                       # Vue components, stores, Typed IPC, DEV Harness
│   ├── tests/                     # Vitest component and business tests
│   └── visual/                    # CUA visual regression scenarios and checks
├── src-tauri/
│   ├── capabilities/main.json     # Main-window permissions
│   ├── src/                       # Command adapter and app assembly
│   ├── tauri.conf.json            # Tauri window and security policy
│   └── tauri.harness.conf.json    # Development IPC Harness configuration
├── tools/ipc-harness/             # Temporary IPC debugging page, not a product UI
├── docs/                          # PRD, technical design, UX, and acceptance records
~~~

<code>target/</code>, <code>src-tauri/gen/</code>, and <code>src-tauri/permissions/autogenerated/</code> are build-generated directories excluded by <code>.gitignore</code>. <code>contracts/v1/</code> contains generated source code that must be reviewed with interface changes; do not clean it as a build cache.

</details>

## Development and quality checks

Frontend and Rust checks run separately. Real OpenSSH and large-file stress tests are started on demand.

<details>
<summary><strong>Show check commands, integration tests, and type generation</strong></summary>

Routine Rust checks:

~~~bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo check --workspace --all-targets --locked
~~~

Frontend type checking, unit tests, and build:

~~~bash
npm --prefix frontend run type-check
npm --prefix frontend run test
npm --prefix frontend run build
node --test frontend/visual/capture.test.mjs
node frontend/visual/verify.mjs
~~~

Real OpenSSH lifecycle, Terminal, and SFTP integration tests are marked ignored by default. They start an isolated loopback OpenSSH fixture and do not use the user's SSH configuration or private keys. Run them on demand, for example:

~~~bash
cargo test -p maulink-core --test openssh_lifecycle --locked -- --ignored --nocapture
cargo test -p maulink-core --test openssh_terminal --locked -- --ignored --nocapture
cargo test -p maulink-core --test openssh_sftp openssh_sftp --locked -- --ignored --nocapture
~~~

SFTP also has long-running stress tests covering 100 MiB, 1 GiB, and 10 GiB files. They use significant disk space and time and are not part of the default test suite. Run them only in a dedicated environment with enough space and time.

Regenerate shared TypeScript types:

~~~bash
cargo run -p maulink-core --example export_bindings --locked
~~~

After generation, review the <code>contracts/v1/</code> diff and confirm it matches the Rust DTOs. <code>tools/ipc-harness/</code> is for development verification only, not a production window. Start it with this separate Tauri configuration:

~~~bash
cargo tauri dev --config src-tauri/tauri.harness.conf.json
~~~

For RustRover's Cargo Run Configuration, set Working directory to the repository root and Command to <code>tauri dev</code>. For a build configuration, use <code>tauri build --bundles app -- --locked</code>. See the [frontend development guide](./frontend/README.md) for the isolated setup and Browser Harness commands.

</details>

## Verification and release status

The acceptance status below is summarized from historical records: backend through **2026-09-28**, and frontend components and interactions through **2026-10-02**. The remote text viewer / editor added on 2026-10-03, the server credential / network information / detail-page updates on 2026-10-08, and the notification-position and server-form updates on 2026-10-09 are not covered by those historical results. The frontend records through 2026-10-02 include type checking, 136/136 tests, Browser keyboard and overlay checks, review of 184 visual screenshots, and macOS arm64 build and signature verification. The local Windows cross-build failed because MSVC SDK headers were missing and remains BLOCKED. System-Dark and system Reduced Motion have not been tested specifically; these results do not represent complete cross-platform QA.

| Scope | Current status | Still needed |
| --- | --- | --- |
| M1–M3: Core foundation, data / credentials, SSH | Core implementation and related unit / isolated OpenSSH tests pass; M3 Windows MSVC cross-compilation passes | Native Windows credential service, network behavior, paths, and real-device installer acceptance |
| M4: Terminal | PTY, multiple terminals, cancellation, and bounded flow control are implemented; local OpenSSH stress scenarios pass | Real WebView / Tauri IPC throughput and latency measurements; until then, performance acceptance is incomplete |
| M5: SFTP | Browsing, directory operations, file transfer, and plain-text viewing / editing up to 2 MiB are implemented; isolated OpenSSH and large-file round-trip checks on 2026-09-28 covered file transfer | Acceptance checks for text viewing / editing and Markdown preview; Windows file publishing and target-server failure matrix |
| M6: Monitor | Collection, parsing, bounded history, and workspace page are implemented; checks in the available environment pass | Compare metrics against Linux hosts; verify minimize / restore behavior on Windows |
| Desktop UI | Production Vue entry point and Reka UI components; Browser visual regression covers both languages and themes; macOS release app confirmed by the user | Existing Phase 6 acceptance gaps, system theme / reduced-motion checks, native minimize / restore, and Windows build environment and desktop acceptance |
| Release | Local development bundle is usable | Developer ID / Windows signing, macOS notarization, DMG, and public release checks |

The current UI provides Simplified Chinese and English catalogs; user names, remote paths, and terminal output remain unchanged. Docker management, database clients, process lists, and Disk I/O monitoring are outside the current MVP. M4 IPC numeric tests were skipped at the user's request. This README does not publish throughput or latency figures that have not been measured.

Development and visual regression guides:

- [Frontend development guide](./frontend/README.md)
- [Visual regression guide](./frontend/visual/README.md)
- [Historical design QA record](./design-qa.md) (through 2026-10-01; does not include the remote text viewer / editor added later)

## Documentation

| Topic | Entry point |
| --- | --- |
| Development and running | [Frontend development guide](./frontend/README.md) · [Project structure](#project-structure) |
| Visual regression | [Visual regression guide](./frontend/visual/README.md) |
| Historical acceptance | [Design QA record](./design-qa.md) (through 2026-10-01) |

## Contribution guidelines

- Keep <code>maulink-core</code> independent of Tauri UI types. Tauri commands adapt the boundary; they must not duplicate business state machines or storage rules.
- For IPC changes, review the Rust DTOs, generated <code>contracts/v1/</code>, Tauri permissions / capabilities, frontend callers, and contract tests.
- Async resources need capacity limits, cancellation paths, and shutdown behavior; do not hide backpressure with unbounded queues.
- Do not commit real server addresses, usernames, keys, credentials, private fixtures, or unsanitized diagnostic data.
- After changes, run Rust and frontend checks that match the scope, and document platform items that have not been accepted in <code>docs/verification/</code>.
- Write Git commit titles and bodies in Simplified Chinese; keep code identifiers, commands, filenames, and technical names in their original form.

## License

The Rust workspace manifest declares <code>MIT OR Apache-2.0</code>, but the repository root does not yet contain the corresponding license texts. Add and confirm them before public release. See the [Lucide license](./frontend/LICENSE-lucide) for third-party icons.
