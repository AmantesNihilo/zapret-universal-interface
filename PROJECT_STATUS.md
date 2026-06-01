# Z2 Project Status

## Implemented

- Tauri 2 desktop shell with Svelte 5 frontend.
- Rust command layer for app state, settings, profiles, presets, services, logs, diagnostics and tests.
- Profile-driven Power workflow for zapret and embedded tg-ws.
- Shared start/stop path for main UI, tray menu and launch autostart.
- Profile JSON storage with active profile switching, creation and deletion.
- Settings JSON storage with startup, tray, theme and accent options.
- Windows startup registration through the current executable.
- Preset discovery with local resources and sibling fallback.
- Favorite/hidden preset preferences.
- Quick preset testing and best-preset batch testing.
- Persisted test results and Activity logs.
- Diagnostics for rights, resources, executables, selected preset, embedded tg-ws engine and tg-ws port.
- `tg-ws-proxy-rs` is vendored under `crates/tg-ws-proxy-rs` and integrated through `src-tauri/src/runtime/tg_ws.rs` instead of launching a separate tray executable.
- Tray menu and close-to-tray handling.
- Debug executable and installer bundle generation.

## Operational Checks

- Use Settings -> Checks after adding resources.
- Run Z2 as Administrator when zapret presets require elevated network changes.
- Keep `resources/zapret` populated for packaged builds, or keep the sibling fallback directory during development.

## Verification Commands

```powershell
cd D:\WORK_DOCUMENTS\.rust\z2
npm.cmd run check
npm.cmd run build
npm.cmd run tauri build -- --debug
```

```powershell
cd D:\WORK_DOCUMENTS\.rust\z2\src-tauri
cargo check
cargo test
```
