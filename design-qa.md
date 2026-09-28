# MauLink UI implementation design QA

**Status: partial — the provided screenshots informed the fixes, but a post-change app capture is unavailable in this environment.**

## References and target

- Design reference: `UI/MauLink_UI_描述文档_v0.1.md` and the source screens in `UI/prototypes/`.
- Add-server comparison: `UI/prototypes/maulink-03-add-server.png` and the user-provided screenshots showing the duplicate titlebar lights, old modal, and target modal.
- Primary screens reviewed: `maulink-01-server-workspace.png`, `maulink-02-welcome-empty-home.png`, `maulink-04-monitor-overview.png`, `maulink-05-settings.png`, `maulink-06-files-transfers.png`, and the connection, credential, edit, and confirmation states 03 and 07–13.
- Target application viewport: 1440 × 920, from `src-tauri/tauri.conf.json`.
- Implementation capture: unavailable. CUA rejected the local `file://` URL because only HTTP(S) is allowed. A local preview server could not bind to 127.0.0.1 in this sandbox, and the browser policy explicitly disallows using another surface or local HTTP preview to work around that rejection.

## Implemented against the references

- Replaced the legacy green dashboard appearance with the prototype’s compact neutral workspace, indigo selection/action states, and light toolbar/sidebar.
- Matched the empty home, server navigation, terminal/files/quick-monitor composition, full Files and Monitor views, Settings categories, theme previews, and existing confirmation dialogs to the shared desktop shell.
- Connected quick-monitor CPU history bars, memory/disk meters, network rates, and load averages to the existing monitor snapshot/history data.
- Kept the existing SSH, SFTP, transfer, settings, and dialog event paths in place.
- Removed the CSS-generated red/yellow/green circles from the in-app toolbar; macOS window controls remain native.
- Reworked the add-server modal to use the prototype's full-width username, segmented authentication choice, password visibility control, compact credential note, collapsed advanced row, and compact footer. Group selection remains available inside Advanced.

## Checks completed

- `node --check frontend/src/main.mjs` and `node --check frontend/src/icons.mjs` — passed.
- `node --test frontend/tests/*.test.mjs` — 9 passed, 0 failed.
- Static markup audit — HTML parser passed; no duplicate IDs, unresolved literal JavaScript ID references, or missing literal icon definitions.
- Static stylesheet audit — balanced CSS delimiters.
- Tauri Release build and arm64 `.app` / ZIP package checks passed after these corrections.

## Not verified

- The attached before/target screenshots made the specific mismatches clear, but there is no post-change app screenshot to compare. Pixel-level spacing, typography, wrapping, overflow, and responsive behavior therefore remain unverified.
- Tauri IPC states were not exercised in a live window. The app’s English preference remains persisted, while the current interface is still predominantly Simplified Chinese.
