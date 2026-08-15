# Контекст проекта ZUI 2.1.1

Актуализировано: 2026-08-15. Этот файл — рабочий источник контекста для дальнейшей разработки в `D:\..FIX_2`.

## 1. Источник истины и идентичность версии

- Актуальная рабочая версия ZUI — `2.1.1`; она основана на локальном снимке `2.1.0` и содержит обновление сетевых движков от 2026-08-15.
- Версия совпадает в `package.json`, `src-tauri/Cargo.toml` и `src-tauri/tauri.conf.json`.
- Старые release assets `2.1.0` являются предыдущим релизом. Assets `2.1.1` — только локальный build candidate для ручной проверки; они не опубликованы и пока не считаются релизом.
- В рабочей папке нет `.git`: это полный зеркальный снимок найденной локальной папки с исходниками, зависимостями, сборочными артефактами и релизами. История коммитов, ветка и diff относительно GitHub здесь недоступны.
- Папки `image/` и `static/` со старыми скриншотами не считаются источником истины о текущем UI. Источник истины — фактическая Svelte-разметка и CSS.
- При расхождении приоритет такой: исполняемый код и конфигурация 2.1.1 → `UPSTREAM_SYNC.md` → release notes → README → ROADMAP/старые изображения.

## 2. Что это за продукт

ZUI (Zapret Universal Interface) — Windows x64 GUI для управления двумя DPI-desync движками семейства `zapret`, подбора их пресетов и встроенного Telegram MTProto/WebSocket-прокси `tg-ws`.

Основная пользовательская задача: не запускать `.bat`-файлы вручную, а выбрать движок и пресет, проверить его на наборе сетевых целей и затем включать/выключать нужные сервисы одной большой кнопкой.

ZUI не является VPN:

- не шифрует весь трафик;
- не скрывает IP;
- не меняет регион;
- не обеспечивает анонимность;
- не гарантирует доступность любого сервиса у любого провайдера.

`zapret` меняет обработку выбранных сетевых потоков так, чтобы затруднить DPI-классификацию. Для `winws`/`winws2` и WinDivert приложение всегда запрашивает права администратора через Windows manifest.

## 3. Что нового и фактически важно в 2.1.x

- Zapret 2 обновлен до официального `bol-van/zapret2` `v1.0.4` x64: `winws2`, WinDivert, Cygwin, Lua runtime, protocol fake payloads и Discord filter.
- Встроенный Rust-порт Telegram proxy обновлен с `1.7.2` до `tg-ws-proxy-rs` `2.2.5-zui.1` (upstream 2.2.4 + parity patch) и сверен с оригинальным Python `Flowseal/tg-ws-proxy` `1.10.0`.
- Добавлены актуальные fallback/pool/cooldown-механизмы tg-ws, Cloudflare Worker routing, test DC, корректная передача больших media-пакетов и domain fronting через `sprinthost.ru`.
- Явный выбор между `Zapret Classic` и `Zapret 2`.
- Раздельные форматы пресетов и процессы движков:
  - Classic: `.bat`/`.cmd` → `winws.exe`;
  - Zapret 2: `.txt` config → `winws2.exe`.
- В комплекте 21 Classic-пресет Flowseal `1.10.1` и 82 Zapret 2-пресета: 70 legacy pack `2.11` плюс 12 проверенных профилей `klondike0x/zapret2-youtube-discord` `v1.0.6`; всего 103 обнаруживаемых встроенных пресета.
- Тесты в текущем UI упрощены до двух режимов: все пресеты выбранного движка или выбранные вручную пресеты.
- Появились устойчивый общий прогресс, фазы запуска/прогрева/проверки/завершения, Passed/Fail и рекомендации.
- Результаты тестов сохраняются, импортируются и экспортируются.
- Можно запускать только `tg-ws`, вообще не настраивая `zapret`.
- Classic и Zapret 2 запускаются скрыто и останавливаются только по PID, принадлежащим ZUI.
- ZUI работает в одном экземпляре: повторный запуск через официальный Tauri single-instance plugin разворачивает и фокусирует уже существующее главное окно.
- Успешный общий Power On записывает `data/power-intent.json`; штатный выход сохраняет намерение, и следующий запуск восстанавливает тот же активный профиль. Явный Power Off удаляет marker.
- 32-символьный tg-ws Secret хранится в `profiles.json`; Telegram-ссылка добавляет к нему стандартный MTProto-префикс `dd`. Пустые legacy Secret мигрируются и сохраняются один раз, runtime больше не создаёт эфемерный ключ.
- Поврежденные JSON восстанавливаются с резервной копией вместо блокировки интерфейса.
- Переработаны журнал активности, диагностика, модальные окна, toast-ошибки, анимации и нижняя навигация.

## 4. Технологический стек

### Desktop/UI

- Tauri 2 — окно, tray, IPC, dialog/opener plugins, упаковка NSIS/MSI.
- Svelte 5 + SvelteKit 2 + TypeScript.
- `@sveltejs/adapter-static`: frontend собирается в статический SPA и встраивается в Tauri.
- Vite 6 — dev/build pipeline.
- Lucide Svelte — иконки.
- Microsoft Edge WebView2 Runtime — рендер UI на Windows.

### Backend/networking

- Rust 2021 для ZUI backend.
- Tokio для асинхронных сетевых задач и embedded `tg-ws`.
- Reqwest + rustls для тестов целей и GitHub updater.
- Serde/serde_json для локальных моделей и хранения.
- Отдельный workspace crate `tg-ws-proxy-rs` `2.2.5-zui.1`, edition 2024.
- В `tg-ws` используются Tokio, WebSocket/TLS, AES-CTR, SHA-256/HMAC, FakeTLS и пул WebSocket-соединений.

### Внешние бинарные ресурсы

- Zapret Classic / `winws.exe` + `WinDivert.dll` + `WinDivert64.sys`.
- Zapret 2 / `winws2.exe`, Lua-скрипты, списки и бинарные шаблоны.
- `tg-ws` не запускает отдельный `tg-ws-proxy.exe`: библиотека встроена в процесс ZUI.

## 5. Архитектура

```text
Svelte UI (src/)
  ├─ App.svelte: главный координатор экранов и действий
  ├─ stores: settings, profiles, presets, state, tests, logs, diagnostics
  ├─ commands.ts: типизированные Tauri invoke-вызовы
  └─ events.ts: подписки на события backend
                │ Tauri IPC/events
                ▼
Rust backend (src-tauri/src/)
  ├─ commands.rs      IPC-команды и валидация входов
  ├─ services.rs      запуск/остановка Classic и Zapret 2
  ├─ tester.rs        пакетное тестирование пресетов
  ├─ presets.rs       discovery, фильтры, favorite/hidden
  ├─ profiles.rs      профили и миграции
  ├─ settings.rs      настройки и Windows autostart
  ├─ diagnostics.rs   проверки окружения
  ├─ report.rs        support report
  ├─ updater.rs       GitHub Releases/update flow
  ├─ tray.rs          системный tray
  ├─ paths.rs         installed/portable/development layout
  └─ runtime/tg_ws.rs встроенный tg-ws runtime
                │
      ┌─────────┴──────────┐
      ▼                    ▼
winws/winws2 + WinDivert   tg-ws-proxy-rs
```

Frontend не работает с файловой системой или процессами напрямую. Он вызывает зарегистрированные Rust-команды, а backend возвращает модели и отправляет события состояния, логов и тестового прогресса.

## 6. Главные пользовательские потоки

### Первый запуск / настройка движка

1. По умолчанию есть профиль `Default`; оба сервиса выключены, движок и пресет не выбраны.
2. UI предлагает `Zapret Classic` или `Zapret 2` и объясняет, что форматы пресетов несовместимы.
3. После выбора движка пользователь переходит к тестам либо выбирает пресет вручную.
4. Смена движка сбрасывает несовместимый выбранный пресет.

### Главная кнопка питания

1. UI читает активный профиль и определяет включенные сервисы.
2. Перед запуском проверяет конфликты и обязательную конфигурацию.
3. Backend запускает `zapret`, `tg-ws` или оба сервиса.
4. Повторное нажатие останавливает только то, чем владеет ZUI.

Важный допустимый сценарий: если `zapret` не настроен/выключен, но `tg-ws` включен, запускается только `tg-ws`. Если включен только ненастроенный `zapret`, запуск блокируется.

### Tray и сворачивание

- Tray содержит Show, Turn on/off, Settings, Quit.
- При первом сворачивании можно выбрать панель задач или tray и запомнить выбор.
- Поддерживаются launch minimized и autostart активного профиля.
- Windows autostart хранится в HKCU Run.

## 7. Как запускаются движки

### Zapret Classic

- Preset discovery рекурсивно ищет `.bat`/`.cmd` в bundled и пользовательских корнях.
- Отбрасываются служебные папки (`bin`, `lists`, `utils`, `blockcheck`, `cygwin`, `docs`, `.git`) и служебные скрипты.
- Пресет обязан иметь ожидаемое имя либо ссылаться на `winws.exe`.
- Перед запуском создается управляемый временный скрипт в `data/runtime`.
- В нем разворачивается `%~dp0`, отключается вызов upstream `service.bat check_updates`, а команды запуска `winws.exe` преобразуются в скрытый управляемый запуск.
- Скрипт запускается через `cmd /D /Q /C` без отдельного консольного окна.

### Zapret 2

- Читаются только файлы верхнего уровня `resources/zapret2/presets/*.txt`, кроме начинающихся с `_`.
- Проверяется наличие обязательных ресурсов, версия `1.0.4` в manifest и Lua marker `NFQWS2_COMPAT_VER_REQUIRED=6`.
- `resources/zapret2/exe/winws2.exe` запускается напрямую.
- Каждая непустая и некомментарная строка config-файла передается как один аргумент. Shell splitting не применяется — пробелы в путях/именах безопасно сохраняются.
- Немедленное завершение процесса считается ошибкой запуска; stdout/stderr попадают в журнал.

### Владение процессами и recovery

- Перед запуском ZUI запоминает уже существующие процессы и определяет новые PID.
- Если найден чужой `winws.exe` или `winws2.exe`, запуск блокируется.
- Собственные PID и движок записываются в `data/runtime/zapret-owned-pids.txt`.
- После аварийного перезапуска ZUI может присоединиться к еще живому собственному процессу.
- Stop применяет taskkill/process APIs только к сохраненным собственным PID.
- Отдельно обнаруживаются известные VPN/proxy-клиенты (Hiddify, Nekobox, sing-box, v2ray, Clash, WireGuard, OpenVPN и др.); UI предлагает отменить, игнорировать или завершить конфликтующие процессы.

## 8. Встроенный tg-ws

`tg-ws` — локальный Telegram MTProto proxy/bridge внутри процесса ZUI. Типовой путь:

```text
Telegram Desktop → локальный MTProto TCP → tg-ws → WSS/TLS 443 → Telegram DC
```

В профиле ZUI доступны:

- host/port (для нового профиля по умолчанию `127.0.0.1:1443`);
- secret и готовая `tg://proxy?...` ссылка;
- редактируемая таблица маршрутов DC → IP;
- встроенный список Cloudflare-доменов;
- свои CF proxy domains;
- один или несколько CF Worker domains;
- реальные CF Proxy/Worker tests по DC 1/2/3/4/5/203; Worker test дополнительно отправляет MTProto-init через туннель;
- SNI для domain fronting (по умолчанию `sprinthost.ru`, пустое поле отключает);
- CF priority и round-robin balance;
- размер TCP socket buffer, размер WS-пула, verbose logging, ротация `tg-ws.log` и force test DC.

Runtime создает отдельный Tokio thread, локальный listener и пул WSS-соединений. При обычной конфигурации используются прямые Telegram DC2/DC4; при включенных CF-маршрутах direct/CF/Worker варианты выбираются по настройкам и fallback-логике.

Библиотека `tg-ws-proxy-rs` умеет больше (inbound FakeTLS, outbound HTTP/SOCKS и upstream MTProto proxies, расширенные таймауты и CLI), но ZUI 2.1.1 пока не выставляет эти server/operator-возможности в профиль: inbound FakeTLS принудительно `None`, outbound и upstream proxy выключены. Все параметры desktop-настроек Flowseal (`host`, `port`, `secret`, `dc_ip`, `verbose`, `log_max_mb`, `buf_kb`, `pool_size`, CF Proxy, custom CF domain, Worker и `force_test_dc`) интегрированы; language/theme, Windows autostart и update-check используются как глобальные настройки ZUI.

## 9. Пресеты

- Встроенные Classic: 21 пользовательский `.bat`, Flowseal `1.10.1`.
- Встроенные Zapret 2: 70 `.txt`, manifest engine `1.0.4`, pack `2.11`.
- Стабильный ID строится из нормализованного пути; для Zapret 2 в hash дополнительно входит тип движка. Старые Classic ID остаются совместимыми.
- Пользователь может подключать дополнительные каталоги Classic-пресетов.
- В UI есть поиск, favorite, hidden, фильтр «только избранные», показ скрытых, открытие каталога и удаление пользовательского корня из настроек.
- Сортировка: избранные выше, скрытые ниже, затем путь без учета регистра.
- Сейчас пользовательские корни подключаются только к Classic discovery. Для Zapret 2 читается фиксированный bundled `resources/zapret2/presets`.

## 10. Система тестирования

Перед тестом ни `winws.exe`, ни `winws2.exe` не должны работать. Для каждого пресета выполняется:

1. запуск выбранного движка;
2. прогрев 2 секунды;
3. параллельные сетевые проверки;
4. остановка движка;
5. подсчет результата, сохранение и событие UI.

### Режимы текущего UI

- «Все»: все пресеты текущего движка, максимум 500.
- «Выбранные»: пользователь отмечает несколько пресетов, максимум 500.

Backend-команды для одиночного quick test и ограниченного best test еще существуют, но текущий `App.svelte` их не использует. Не следует считать их доступными пользователю без отдельного UI.

### Цели и проверки

- По умолчанию 17 целей: Discord, YouTube, Google, Cloudflare и DNS.
- 12 URL-целей получают HTTP/1.1, TLS 1.2, TLS 1.3 и ping hostname.
- 5 явных `PING:`-целей получают ping.
- Итого при доступных TLS clients — 53 проверки на один пресет.
- Одновременно выполняется максимум 8 проверок.
- HTTP timeout 5 секунд, redirects до 4, сначала HEAD, при необходимости GET; есть повторная попытка.
- Windows ping: 3 пакета, timeout 1500 ms.
- Если в Settings задан хотя бы один валидный включенный target, пользовательский набор полностью заменяет defaults.

### Оценка

- `score = passed / total * 100` (целое число).
- `recommended`: score >= 70 и Discord+YouTube не провалились оба полностью.
- `partial`: score >= 35 либо прошла хотя бы одна проверка.
- иначе `notRecommended`.
- Результаты сортируются по score, затем passed count, затем имени пресета.
- В runtime держится максимум 100 результатов; импортированный файл после merge ограничивается 500.
- Результаты содержат engine, guessed preset version, mode, cachedAt и детализацию по сервисам/целям.

Тест — эвристика, не доказательство реальной работы клиента. Discord voice, browser cache, другие CDN/endpoints, DNS и региональные особенности могут дать иной результат.

## 11. Интерфейс 2.1.1

Окно — собственное без стандартной Windows-рамки, нерастягиваемое и немаксимизируемое:

- portrait: `440 × 700` (default и minimum);
- landscape: `920 × 560`;
- верхняя drag-панель: status badge, `ZUI`, About, minimize, close;
- главный экран: крупная круглая анимированная power-кнопка, пояснение состояния, карточки `zapret` и `tg-ws`, компактный segmented dock «Настройки / Активность»;
- фон и панели темные, почти графитовые, с мягкими границами/тенями; акцент по умолчанию яркий cyan `#25c7d9`;
- UI использует Segoe UI Variable, небольшие радиусы (6–8 px), pill/status элементы, restrained fly/slide transitions и анимированные power rings;
- ошибки — компактные toast-уведомления поверх интерфейса, auto-dismiss около 9 секунд.

### Экраны

- Main: power, две service cards, быстрый переход к настройкам и журналу.
- Settings → General: движок, theme, language, orientation, minimize behavior, accent, autostart, updater.
- Settings → Services: enable/configure Zapret, engine/preset, rescan/custom root; enable/configure tg-ws и advanced CF routing.
- Settings → Test: onboarding выбора движка, summary/progress, два режима теста, сохраненные рекомендации, редактор целей, import/export.
- Settings → Presets: статистика, search, favorite/hidden, custom roots, reveal.
- Settings → Checks: diagnostics и support report.
- Activity: три канала — App (включая tests), текущий Zapret engine и tg-ws; журнал можно очистить.

### Темы и локализация

- Themes: Dark, Light, OLED, System.
- Accents: cyan, teal, green, lime, blue, violet, pink, red, orange, amber.
- Languages: русский и английский.
- Default: Dark + cyan + русский + portrait.

### Основные модальные состояния

- выбор/смена Zapret engine;
- выбор нескольких пресетов для теста;
- редактор целей;
- подробности теста;
- конфликтующие приложения;
- выбор поведения minimize;
- About и update.

## 12. Локальные данные

Файлы:

```text
settings.json
profiles.json
preset-preferences.json
test-results.json
logs/
runtime/zapret-owned-pids.txt
```

Размещение:

- portable: `<папка ZUI>/data`, определяется по `portable.flag`;
- development: `<project root>/data`;
- installed: `%APPDATA%\ZUI` (fallback `%LOCALAPPDATA%\ZUI`).

JSON parser убирает UTF-8 BOM. Пустой/поврежденный JSON переименовывается в `.bad-<timestamp>`, после чего создаются defaults. Это применяется к settings, profiles, preferences и test results.

Профили мигрируют старое состояние без engine в Classic, гарантируют непустой список и валидный active profile. По умолчанию оба сервиса выключены — release portable безопасно не стартует сетевые сервисы сам.

## 13. Диагностика, журнал и отчет

Diagnostics сообщает:

- пути resources/data/logs;
- число пресетов;
- наличие выбранного пресета;
- наличие и состояние `winws.exe` и `winws2.exe`;
- embedded tg-ws engine/version;
- admin status (`net session`);
- доступность порта tg-ws;
- warnings.

Support report содержит версию/distribution mode, состояния сервисов, выбранный engine/preset, diagnostics и последние 80 строк журнала. tg-ws secret в отчет не включается.

Логи делятся на App, Zapret, tg-ws и Tests и одновременно отправляются в UI событиями.

## 14. Updater и дистрибуция

- Источник обновлений: GitHub Releases проекта `AmantesNihilo/zapret-universal-interface`.
- Installed build может скачать setup asset, корректно остановить сервисы, запустить installer и выйти.
- Portable/development build только сообщает о версии и открывает страницу/asset вручную.
- Открываемые внешние URL проверяются backend allowlist; тесты покрывают разрешенные project URLs и отказ для остальных.
- Release-артефакты: NSIS setup, MSI, portable ZIP и `SHA256SUMS.txt`.
- `tools/build-release.ps1` собирает portable staging, создает безопасные defaults с выключенными сервисами, архивирует и рассчитывает SHA-256.

## 15. Карта важных файлов

```text
src/app/App.svelte                  главный UI и orchestration
src/lib/styles/tokens.css          themes/accent/design tokens
src/lib/styles/global.css          вся визуальная система и responsive layout
src/lib/api/commands.ts            frontend → Rust command contract
src/lib/api/events.ts              Rust → frontend event contract
src/lib/api/types.ts               TS-модели IPC
src/lib/stores/*                   состояние UI
src/lib/components/*               power, services, presets, tests, logs, diagnostics

src-tauri/src/lib.rs               Tauri builder, setup и command registry
src-tauri/src/commands.rs          публичный IPC слой
src-tauri/src/services.rs          процессы Zapret Classic/Zapret 2
src-tauri/src/tester.rs            test runner/scoring/persistence
src-tauri/src/presets.rs           discovery и preferences
src-tauri/src/models.rs            Rust domain models
src-tauri/src/paths.rs             distribution/data/resource paths
src-tauri/src/json_storage.rs      BOM/corrupt JSON recovery
src-tauri/src/runtime/tg_ws.rs     adapter Profile → embedded tg-ws Config
src-tauri/src/updater.rs           GitHub update flow и URL validation
src-tauri/src/diagnostics.rs       checks
src-tauri/src/report.rs            support report
src-tauri/src/tray.rs              tray lifecycle/actions
src-tauri/build.rs                 requireAdministrator manifest

crates/tg-ws-proxy-rs/             встроенный Telegram proxy engine 2.2.5-zui.1
resources/zapret/                  Classic resources/presets
resources/zapret2/                 Zapret 2 engine/resources/70 configs
release/                            release assets (2.1.0 — предыдущие, 2.1.1 — текущие)
tools/build-release.ps1            упаковка релиза
```

## 16. Проверки текущего снимка

Выполнено 2026-08-15:

- `npm.cmd run check` — PASS, 0 errors, 0 warnings.
- `cargo test --manifest-path src-tauri\Cargo.toml --lib` — PASS, 25/25.
- `cargo test --manifest-path crates\tg-ws-proxy-rs\Cargo.toml` — PASS: 188 passed, 2 Windows DNS-only tests ignored, 0 failed; doc tests PASS. Эти два теста выполняются на платформах, где resolver поддерживает subdomain `localhost`.
- Live CF check через реальный сетевой код — PASS: один автоматический Flowseal domain успешно прошёл DC 1/2/3/4/5/203; отдельный стандартный `--check --default-domains` успешно проверил 20/20 актуальных доменов на DC2.
- `cargo clippy --manifest-path crates\tg-ws-proxy-rs\Cargo.toml --all-targets` — PASS с одним upstream test-only warning `result_large_err`.
- Полный `cargo test` для Tauri crate собирает библиотеку и успешно прогоняет 25 tests, но затем Windows отказывается запускать test harness бинарника `zui.exe` без UAC (`os error 740`). Причина — намеренный `requireAdministrator` в `build.rs`, а не падение library tests.
- `npm run tauri build` — PASS; NSIS и MSI `2.1.1` собраны.
- `tools/build-release.ps1 -Version 2.1.1` после текущей правки намеренно не запускался: portable/release package остаётся предыдущим и не должен использоваться для проверки нового tg-ws UI.
- Новый локальный EXE сообщает ProductVersion/FileVersion `2.1.1`; публикация, tag и GitHub Release не выполнялись.
- Локальный listener, Telegram-клиент и реальные DPI-сценарии пока требуют ручной проверки; live CF probes выполнялись без изменения системной маршрутизации.

SHA-256 локальной сборки после переноса настроек tg-ws (ещё не релиз):

- application EXE: `900a8618f524778c2ae95bbd7a0dbf2669a757de07c6313d59930f3d66c6e04f`;
- NSIS: `5248a4c88f2f0b76bbee1bb304ced0e88a3c2b5d2bb9fbe3c0d3a68ca3225e20`;
- MSI: `827c1212e6f1e883d8e92976917170d408f075e66d730de091fdfd1e105f5c8f`.

## 17. Известный техдолг и риски 2.1.1

1. README и ROADMAP не полностью соответствуют коду 2.1.1:
   - продолжают упоминать quick/full/best UI;
   - структура проекта не перечисляет `resources/zapret2`;
   - ROADMAP помечает будущими некоторые уже реализованные функции.
2. HTTP test client отправляет устаревший User-Agent `ZUI/2.0`.
3. `tauri.conf.json` содержит `"csp": null`. Для всегда elevated desktop-приложения это важный hardening-кандидат.
4. Tauri capability включает `opener:default`; вместе с отсутствующей CSP область открытия URL нужно пересмотреть, даже несмотря на backend allowlist собственных `open_url` команд.
5. `npm audit` на текущем lockfile: 5 уязвимых пакетов (3 high, 1 moderate, 1 low), fixes available. Затронуты Vite, PostCSS, nanoid, SvelteKit и cookie. Большинство относится к dev/server/build paths, а packaged frontend статический, но зависимости следует обновить и повторно проверить.
6. Полный Rust test command неудобен для CI из-за UAC-манифеста test binary; штатной проверкой сейчас является `cargo test ... --lib` плюс отдельные tg-ws tests.
7. `App.svelte` (~88 KB) и `global.css` (~4.7k строк) монолитны; дальнейшие крупные UI-изменения лучше сопровождать выделением экранов/стилей по модулям.
8. `Cargo.toml` содержит placeholder `authors = ["you"]`.
9. Нынешняя рабочая папка без Git metadata и содержит `node_modules`, `.svelte-kit`, `build`, Rust `target`, local `data`, `release` и старые `image`. Перед нормальной командной разработкой нужен осознанный выбор: восстановить Git history или инициализировать новый репозиторий, не потеряв этот снимок.

## 18. Правила для следующей работы

- Считать этот снимок и код 2.1.1 базой; не переносить решения из 2.0.x без проверки diff/контрактов.
- Не использовать изображения из `image/` как UI reference.
- Не запускать preset tests, `winws`, `winws2`, tg-ws или installer без явной необходимости: это elevated/network-changing действия.
- При изменении профиля сохранять обратную совместимость JSON и recovery поведения.
- При изменении engine/preset всегда обеспечивать совпадение `Profile.zapretEngine` и `Preset.engine`.
- При изменении process lifecycle сохранять главный invariant: ZUI останавливает только собственные PID.
- При изменении тестов сохранять стабильный total checks и корректный cleanup при cancel/panic.
- После правок запускать минимум frontend check, Tauri lib tests и tg-ws tests; для релиза — полный checklist/build и smoke tests installed/portable.
- Документацию и release notes сверять с фактическим UI, особенно режимы тестов и поддерживаемые engine formats.
