# ZUI Release Checklist

Use this checklist before every public build.

## 1. Code and Data

- [ ] Version is correct in `package.json`, `src-tauri/Cargo.toml` and `src-tauri/tauri.conf.json`.
- [ ] `README.md` describes the same version and release assets.
- [ ] `ROADMAP.md` is updated when release scope changes.
- [ ] `.gitignore` excludes `data/`, `release/`, build output and local logs.
- [ ] Bundled zapret resources are present in `resources/zapret`.
- [ ] No local runtime data, logs, cache files or user paths are committed.

## 2. Checks

- [ ] `npm.cmd run check` passes.
- [ ] Rust tests pass for the Tauri crate.
- [ ] Rust tests pass for `crates/tg-ws-proxy-rs`.
- [ ] `npm.cmd run tauri build` creates NSIS and MSI bundles.
- [ ] Portable package is rebuilt from the release executable and bundled resources.
- [ ] SHA256 checksums are regenerated for every public asset.

## 3. Installed Build Scenario

- [ ] Installer starts with the ZUI icon and custom installer artwork.
- [ ] Installed app starts as administrator.
- [ ] App window, taskbar, tray, installer and uninstaller icons look correct.
- [ ] First launch creates user data in `%APPDATA%\ZUI`.
- [ ] Update check runs on launch and shows changelog only when an update exists.
- [ ] Installed update downloads the setup asset, stops services, launches installer and exits ZUI.
- [ ] Uninstall closes running ZUI, stops ZUI-owned `winws.exe`, asks whether to remove user data and finishes without hanging.

## 4. Portable Build Scenario

- [ ] `portable.flag` is present next to `ZUI.exe`.
- [ ] Portable app stores data beside the executable in `data/`.
- [ ] Portable update check opens GitHub release/download instead of auto-installing.
- [ ] Removing the portable folder after closing ZUI leaves no running ZUI-owned `winws.exe`.

## 5. Runtime Behavior

- [ ] Power button is disabled when no runnable service or preset is selected.
- [ ] Zapret starts hidden and stops only the `winws.exe` processes started by ZUI.
- [ ] Foreign `winws.exe` is detected before launch.
- [ ] tg-ws starts without a separate tray/window and opens a local Telegram proxy link.
- [ ] Test stop overlay blocks user actions until cleanup finishes.
- [ ] Crash recovery detects a previous ZUI-owned `winws.exe` and lets the user stop it.
- [ ] Diagnostics show resources, data path, logs path, admin state, tg-ws port and warnings.
- [ ] Support report includes version, distribution, selected preset, service states and recent logs.

## 6. Release Assets

- [ ] `ZUI_<version>_x64-setup.exe`
- [ ] `ZUI_<version>_x64_en-US.msi`
- [ ] `ZUI_<version>_portable.zip`
- [ ] `SHA256SUMS.txt`
- [ ] `release-notes.txt`

## 7. GitHub Release

- [ ] Tag uses the version format expected by the updater, for example `v2.0.1`.
- [ ] Release notes include changes, fixes, known limitations and install/update notes.
- [ ] Assets are uploaded after checksum generation.
- [ ] A short warning is included: ZUI is not a VPN and cannot guarantee every service in every region.
- [ ] Smoke test is repeated after downloading assets from GitHub Releases.
