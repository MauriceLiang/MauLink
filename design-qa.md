# MauLink UI implementation design QA

**Delivery status: source-aligned implementation and macOS app build are ready for the user's visual check.**

## Design references

- `UI/slides.pdf` contains 34 English and Simplified Chinese screens, including workspace, empty home, server dialogs, monitor, settings, file management, and connection/security states.
- `UI设计.pdf` is the 68-page export of the same English and Chinese screen set; `UI/页面原型.html` supplies the editable source layout and component styling.
- Desktop artboards are 1440 × 900. The Tauri window is configured for 1440 × 920 with a 48 px shared toolbar and native macOS traffic lights.

## Updated implementation

- The native macOS close, minimize, and zoom controls overlay the shared brand toolbar beside the MauLink logo; no separate black title strip or CSS-drawn traffic lights are used.
- The toolbar, sidebar, server workspace, terminal/files split, quick-monitor column, full Files and Monitor views, settings shell, and Chinese/English labels follow the supplied artboards. The quick-monitor column spans the terminal and file-list rows.
- The add-server form follows the supplied order and dimensions. Its footer contains Test Connection and Connect; Edit Server contains Cancel and Save Changes plus the separate delete action. The new design has no visible Group field, so editing keeps the existing group assignment in a hidden form value.
- Advanced connection settings use a full-width Jump Host field, an 88 px Jump Port field beside Proxy, and full-width Keep Alive. The card follows the reference neutral fill, 1 px border, 10 px radius, and 16 px inset.
- Jump-host and proxy values persist through the versioned SQLite migration, TypeScript contracts, Tauri commands, and SSH connection manager. Connections support one SSH jump host and no-auth SOCKS5 or HTTP CONNECT; a configured proxy can carry either the jump-host connection or a direct target connection.
- Static UI strings, accessibility labels, placeholders, titles, and runtime messages have English mappings. Monitor status and app-generated terminal notices follow the selected language; server names, addresses, remote paths, and remote terminal output remain literal.

## Known gaps

- The source prototype requests Inter and JetBrains Mono from Google Fonts. Those font files are not bundled with the app, and the Tauri content security policy blocks remote fonts; system font fallbacks are used. Exact text metrics may differ from the prototype.
- No post-change macOS window screenshot is available. Exact visual parity, text wrapping, and native traffic-light alignment remain for the user's manual check in the delivered app.
- SOCKS5 and HTTP CONNECT handshake tests use in-memory streams. The ignored local OpenSSH jump-host integration test was attempted, but its fixture could not reserve a loopback port (`Operation not permitted`). A live SSH/SFTP path and Tauri IPC were not exercised. The user chose to skip the MauLink Backend Harness IPC measurement after its launch was not approved.
- Proxy authentication is not shown in the supplied design and is not implemented. Jump-host authentication reuses the profile's credential and private key, with an optional `user@host` username override.

## Checks performed

- `node --check frontend/src/main.mjs` and `node --check frontend/src/i18n.mjs` — passed.
- `node --test frontend/tests/*.test.mjs` — 13 passed, 0 failed.
- `cargo fmt --all -- --check` — passed.
- Static markup audit — 242 unique IDs, no duplicates, and no unresolved literal `main.mjs` ID selectors.
- Static bilingual audit — all 202 unique Chinese text, ARIA, placeholder, and title values in `frontend/index.html` translate to English.
- `cargo test --package maulink-core --locked --offline` — 64 unit tests and 4 contract tests passed; 1 native credential-store test and OpenSSH integration tests are ignored by default.
- `cargo check --package maulink-desktop --locked --offline` — passed.
- `cargo build --release --package maulink-desktop --locked --offline` — passed; the release executable was installed into `target/release/bundle/macos/MauLink.app` and ad-hoc signed for local testing.
- `codesign --verify --deep --strict --verbose=2 target/release/bundle/macos/MauLink.app` and `plutil -lint .../Contents/Info.plist` — passed.
- `git diff --check` — passed.
