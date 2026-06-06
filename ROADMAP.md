# ZUI Roadmap

## 2.0.x - stabilization after first release

Goal: make the app reliable for users who just want to download, launch, select a preset and run it.

- Fix uninstall flow: close ZUI, stop owned processes, and ask whether to remove or keep user data.
- Polish update flow: changelog, postpone action, installed auto-update, portable manual download.
- Test installed and portable builds as separate scenarios.
- Verify icons for app window, taskbar, tray, installer and uninstaller.
- Add crash recovery after incorrect shutdown.
- Improve logs: app, zapret, tg-ws and tests.
- Add "collect report" action with version, mode, selected preset, process state and recent logs.
- Maintain a release checklist for every public build.

## 2.1 - tests and diagnostics

Goal: help users understand why a preset works, partially works, or fails.

- Rework the Test tab into a clearer flow.
- Keep separate modes for quick test, full test and best preset search.
- Show per-service details for Discord, YouTube, Cloudflare and Google.
- Add clear result states: recommended, partial, not recommended.
- Explain that test results are not always equal to real browser or Discord behavior.
- Add configurable test targets.
- Cache test results with date and preset version.
- Add "apply best preset" action.
- Allow exporting test results.

## 2.2 - presets and sources

Goal: make ZUI a comfortable preset control center.

- Add a preset source manager.
- Support bundled sources, Flowseal presets, Howdyho/DiscordFIX targets and user folders.
- Update presets separately from app updates.
- Strictly separate presets from service, utils, bin, lists, blockcheck and other non-preset files.
- Improve favorite and hidden preset handling.
- Allow removing user preset folders from the app in one click.
- Add settings import and export.
- Add recommendations: often working, recently successful, Discord-focused, YouTube-focused.
- Keep recent preset history.

## 2.3 - services and launch flow

Goal: make ZUI control only the processes it started and avoid conflicts.

- Track owned winws and tg-ws process IDs.
- Detect foreign winws before launch.
- Detect conflicting VPN/proxy clients such as Hiddify, Nekobox and sing-box.
- Offer conflict actions: cancel, ignore or terminate conflicting processes.
- Block unrelated UI actions while services are starting or stopping.
- Add polished waiting overlay for launch, stop and test shutdown.
- Make zapret stop flow more reliable.
- Request administrator rights earlier and more clearly.
- Add dedicated diagnostics for occupied ports.

## 2.4 - UX polish

Goal: make the app feel finished in both vertical and horizontal layouts.

- Polish vertical and horizontal modes.
- Use two-column settings layouts where horizontal mode benefits from it.
- Add a short first-launch setup wizard.
- Redesign the Checks tab into a cleaner diagnostics panel.
- Improve empty states for no presets, no admin rights, occupied port and no selected service.
- Unify dropdowns, checkboxes, toggles and badges.
- Tune fonts, spacing, contrast and long Russian labels.
- Bring OLED, Dark, Light and System themes to the same quality level.
- Add more accent colors.
- Keep animations useful and restrained.

## 2.5 - security and trust

Goal: make releases easier to trust and verify.

- Sign builds when a certificate becomes available.
- Publish SHA256 checksums for every release.
- Keep a visible "ZUI is not a VPN" warning.
- Document why administrator rights are required.
- Explain possible antivirus false positives.
- Keep source build instructions independent from local developer paths.
- Add GitHub Actions checks.
- Add release candidate builds through GitHub Actions.
- Consider stable, beta and nightly channels if needed.

## 3.0 - mature ZUI

Goal: turn ZUI into a complete zapret control center.

- Add automatic preset selection for the user.
- Add region-aware recommendations.
- Add simple scenarios instead of heavy profiles: Discord, YouTube, Telegram, Games.
- Auto-start only if previous launch was successful.
- Add backup and restore for settings.
- Add advanced mode for experienced users.
- Keep the main screen simple for regular users.
- Consider a small website or landing page with downloads, screenshots and FAQ.

## Distribution

- Use GitHub Releases as the primary source.
- Keep README as the main landing page: download, what to choose, screenshots, FAQ and limitations.
- Use Telegram channel or chat for release announcements and feedback.
- Share short GIFs or videos: install, select preset, run tests.
- Maintain `CHANGELOG.md`.
- Add GitHub issue templates for bugs, preset issues, installation issues and feature requests.
- Publish installer, MSI, portable ZIP and SHA256 for every release.
- Consider Winget later, after the app is more stable and signed.

## Suggested priority

1. 2.0.1 Stability Patch
2. 2.1 Test & Diagnostics
3. 2.2 Preset Sources
4. 2.3 Service Control & Conflicts
5. 2.4 UX Polish
6. 2.5 Release Pipeline
