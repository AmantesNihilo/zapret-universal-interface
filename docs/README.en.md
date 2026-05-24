<div align="center">

<img src="../gui/ui/assets/Z2.png" width="96" alt="Z2 GUI logo">

# Z2 GUI

**Portable Windows GUI for zapret2 presets, automatic testing, one-click launch, and tg-ws-proxy.**

[![Platform](https://img.shields.io/badge/platform-Windows-0078D4)](#requirements)
[![GUI](https://img.shields.io/badge/GUI-PySide6-41CD52)](#build-from-source)
[![Build](https://img.shields.io/badge/build-portable-blue)](#build-from-source)
[![License](https://img.shields.io/badge/license-see_credits-lightgrey)](#credits-and-licenses)

[Русский](README.md) | English

</div>

Z2 GUI is a portable Windows application for finding, testing, saving, and launching zapret2 presets without manually editing scripts or running batch files.

It also includes source-based `tg-ws-proxy` integration: the app can start a local Telegram MTProto WebSocket proxy, copy the proxy link, and open it directly in Telegram Desktop.

## Screenshots

| Home | Presets | Settings |
| --- | --- | --- |
| <img src="../gui/ui/assets/Home.jpg" alt="Home page" width="320"> | <img src="../gui/ui/assets/Preset.jpg" alt="Presets page" width="320"> | <img src="../gui/ui/assets/Setting.jpg" alt="Settings page" width="320"> |

## What It Is

| Z2 GUI is | Z2 GUI is not |
| --- | --- |
| A GUI wrapper around zapret2 presets and runtime tools. | A VPN or a full traffic tunnel. |
| A local preset tester and launcher. | A guarantee that every service will work in every network. |
| A convenient way to manage `winws2` and `tg-ws-proxy`. | A replacement for understanding what you run on your PC. |

> [!IMPORTANT]
> Z2 GUI is not a VPN. It manages local DPI-bypass tools and a local Telegram proxy. Use it only where it is allowed by your network rules and local law.

> [!WARNING]
> zapret requires administrator rights because `winws2.exe` and WinDivert work with network packets. Antivirus software may flag WinDivert, networking tools, or PyInstaller builds heuristically. Download builds only from trusted sources.

> [!CAUTION]
> Do not run unknown repackaged builds unless you trust the distributor. The portable folder contains executables, driver components, presets, logs, and local settings.

## Contents

- [Screenshots](#screenshots)
- [Features](#features)
- [Requirements](#requirements)
- [Quick Start](#quick-start)
- [Administrator Rights](#administrator-rights)
- [Testing Presets](#testing-presets)
- [Favorites](#favorites)
- [tg-ws-proxy](#tg-ws-proxy)
- [Activity Logs](#activity-logs)
- [Troubleshooting](#troubleshooting)
- [Build From Source](#build-from-source)
- [Project Structure](#project-structure)
- [Security Notes](#security-notes)
- [Support the Author](#support-the-author)
- [Forks and Attribution](#forks-and-attribution)
- [Credits and Licenses](#credits-and-licenses)

## Features

| Area | What Z2 GUI does |
| --- | --- |
| Presets | Preset table, search, filters, favorites, custom presets, hide/delete actions. |
| Testing | Automatic checks for selected presets using Discord, YouTube, and other configurable targets. |
| Results | Last test results are restored after restart and sorted by `Score`. |
| Launch | Home screen with favorite presets and a single Start / Stop flow. |
| tg-ws | Local Telegram MTProto WebSocket proxy with port, secret, link copy, Open in Telegram, and restart actions. |
| Activity | Separate consoles for zapret and tg-ws logs. |
| Safety | VPN/proxy conflict detection before tests and launches. |
| UI | Fluent-like interface with Windows 11, Dark, AMOLED, and Light themes. |
| Packaging | Portable build; Python is not required on the target PC. |

## Requirements

- Windows 10/11 recommended.
- Administrator rights for zapret / `winws2`.
- Telegram Desktop if you want to use `tg-ws-proxy`.
- Internet access for real target checks.

> [!NOTE]
> `tg-ws-proxy` does not require administrator rights by itself. If you start it together with zapret from the Home button, the app checks permissions first to avoid a partial launch.

## Quick Start

1. Download or build the portable package.
2. Open the unpacked folder.
3. Run the executable. Windows should show a UAC prompt automatically:

```text
Zapret2GUI.exe
```

4. Open `Presets` and add one or more presets to Favorites with the star button.
5. Optional: run `Test Selected` to find the best preset by `Score`.
6. Return to `Home`, choose a favorite preset, and press `Start`.

> [!TIP]
> Home intentionally shows only Favorites. This keeps the launch screen clean and prevents accidental launches from a huge preset list.

## Administrator Rights

zapret2 uses `winws2.exe` and WinDivert. These tools inspect and modify network packets, so Windows requires elevated privileges.

Without administrator rights you can still open Z2 GUI, browse presets, edit settings, and configure tg-ws. Starting or stopping zapret may fail without elevation.

## Testing Presets

1. Open `Presets`.
2. Select rows with checkboxes in the `Test` column.
3. Open `Test Settings` and keep only the targets you need, for example Discord only or YouTube only.
4. Press `Test Selected`.
5. Watch `Activity -> zapret` while the test is running.
6. Sort by `Score` and add the best presets to Favorites.

Result columns:

| Column | Meaning |
| --- | --- |
| OK | Passed target checks. |
| FAIL | Failed checks. |
| UNSUP | Unsupported checks. |
| Ping | Passed ping checks / total ping checks. |
| Score | Combined preset rating. Higher is usually better. |

Last results are stored locally:

```text
utils/gui-results.json
```

## Favorites

A preset becomes a Favorite when you click the star in the `Fav` column.

Favorites are stored locally:

```text
utils/gui-settings.json
```

Only Favorites are shown in the Home preset dropdown.

## tg-ws-proxy

Z2 GUI includes source-based integration with `tg-ws-proxy`. It is launched as a helper process of Z2 GUI, not as a separate downloaded executable.

### How to Enable

1. Open `TG`.
2. Check the port. Default endpoint:

```text
127.0.0.1:1443
```

3. Keep the generated `Secret` or set your own 32-character hex secret.
4. Press `Restart tg-ws`.
5. Press `Open in Telegram` or `Copy proxy link`.

### Home Integration

The Home page has a `tg-ws` card. The `Start with main button` switch controls whether tg-ws participates in the main `Start` button.

You can run only zapret, only tg-ws, or both together.

### Logs

tg-ws log file:

```text
utils/tg-ws-proxy.log
```

You can open it from the `TG` page or read the tail in `Activity -> tg-ws`.

## Activity Logs

The `Activity` page has two consoles:

- `zapret` - app events, preset tests, winws2 start/stop, conflict checks;
- `tg-ws` - tg-ws events and recent lines from `utils/tg-ws-proxy.log`.

`Clear Log` clears only the currently selected console.

## Troubleshooting

| Problem | What to do |
| --- | --- |
| zapret does not start | Run Z2 GUI as administrator. |
| Preset exits immediately | Try another preset or check the preset file. |
| Tests take too long | Open `Test Settings` and keep only the required targets. |
| Discord or YouTube still does not work | Stop the current preset, test several other presets, choose a higher `Score`. |
| VPN/proxy conflict detected | Close the listed client from the dialog or manually. VPN/proxy clients often intercept traffic and can break zapret. |
| tg-ws does not start | Check whether port `1443` is busy, or change the port on the `TG` page. |
| Telegram does not open the proxy link | Use `Copy proxy link`, send the link to yourself in Telegram, and click it manually. |
| Antivirus warning | Check the source, build yourself if needed, and remember that WinDivert/network tools are often flagged heuristically. |

## Build From Source

Install dependencies:

```powershell
python -m pip install -r requirements.txt
```

Run from source:

```powershell
python .\gui\app.py
```

Build portable package:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\build-exe.ps1
```

Output:

```text
dist/Zapret2GUI/Zapret2GUI.exe
```

Keep the generated portable folder intact. Moving only `Zapret2GUI.exe` without bundled files can break presets, WinDivert, icons, logs, licenses, or tg-ws integration.

## Project Structure

```text
gui/                 GUI and app logic
gui/zapret_core/     presets, testing, winws2, tg-ws managers
presets/             bundled presets
utils/               settings, results, logs, runtime files
exe/                 winws2 and WinDivert files
vendor/              vendored source code for third-party components
lists/               host/ip lists
lua/                 zapret Lua scripts
```

## Security Notes

Z2 GUI works with networking tools, so treat builds seriously:

- prefer official or self-built releases;
- verify that the portable folder was not modified by an unknown third party;
- expect antivirus heuristics around WinDivert, `winws2`, packet tools, or PyInstaller;
- close VPN/proxy clients if the app reports a conflict;
- do not share your `tg-ws` secret publicly;
- review custom presets before adding them.

## Support the Author

If Z2 GUI is useful to you, you can support the author. This is optional, but it helps with development time, preset testing, and release maintenance.

| Method | Details |
| --- | --- |
| Bank card | `4377 7278 0187 1414` - Daniil P. |
| Solana | `1hHoDcgWEWF96Yy97hes2gUoSkgANkAE1kNPnJ9Z9Uq` |
| Ethereum | `0x7B30eEE5C1625a754915cf761eD7D0DF24A97107` |
| Bitcoin | `bc1qv6x8677487qhkrz50mmx9ymsyagngzfp6fa58j` |

> [!NOTE]
> Before sending anything, verify payment details only from the official project README or release page. Do not trust donation details from random repackaged builds.

## Forks and Attribution

If you fork, repackage, redistribute, or publish a modified build of Z2 GUI:

- keep visible credit to `Z2 GUI` and author `amantesnihilo`;
- keep credits for upstream projects listed below;
- do not present upstream work as your own;
- clearly mark your fork as modified if you change behavior, presets, binaries, or bundled components;
- keep license files and attribution notices for bundled third-party projects;
- do not remove warnings about administrator rights, WinDivert, antivirus reactions, or VPN/proxy conflicts;
- do not publish a rebranded build in a way that hides the original project and upstream authors.

A good fork notice looks like this:

```text
Based on Z2 GUI by amantesnihilo.
Uses zapret2 by bol-van, zapret2-youtube-discord by youtubediscord,
and tg-ws-proxy by Flowseal.
```

Recommended placement for attribution:

- README;
- About / Credits screen inside the app;
- release page or archive description;
- installer page, if you create an installer.

## Credits and Licenses

Z2 GUI brings together several upstream projects:

| Project | Role | Credit |
| --- | --- | --- |
| zapret2 | DPI bypass toolkit / winws2 base | https://github.com/bol-van/zapret2 |
| zapret2-youtube-discord | preset source and zapret2 bundle | https://github.com/youtubediscord/zapret2-youtube-discord |
| tg-ws-proxy | Telegram MTProto WebSocket proxy | Flowseal, MIT License, https://github.com/Flowseal/tg-ws-proxy |

Z2 GUI author: `amantesnihilo`.

See also: [NOTICE.md](NOTICE.md).

The upstream projects keep their own licenses and attribution requirements. If you redistribute a build, keep their license files and notices together with the application.

> [!CAUTION]
> This project is provided as-is. You are responsible for how and where you use it. Test presets carefully, keep backups of custom presets, and do not run unknown builds from untrusted sources.
