# Синхронизация upstream-движков ZUI 2.1.1

Дата синхронизации: 2026-08-15.

## Зафиксированные источники

| Компонент | Версия | Commit |
| --- | --- | --- |
| Zapret 2 | `bol-van/zapret2` `v1.0.4` | `2c21faa80e1acb71ddceb8b49176f266b7d33f05` |
| Zapret 2 preset additions | `klondike0x/zapret2-youtube-discord` `v1.0.6` | `f97a4b069f265d813aa83a304786f1ff989eab3f` |
| Zapret Classic presets | `Flowseal/zapret-discord-youtube` `1.10.1` | `47da17f80ad36a8424cdd25658153fdebd7eb938` |
| Оригинальный tg-ws | `Flowseal/tg-ws-proxy` `v1.10.0` | `b2a8074c59c52cabde7fe295280b614cc6c01fce` |
| Upstream Rust-порт tg-ws | `valnesfjord/tg-ws-proxy-rs` `v2.2.4` | `641c6b6bd028b17c1f238f9339103345c2887b66` |
| Встроенный ZUI-вариант | `tg-ws-proxy-rs` `2.2.5-zui.1` | upstream tag + Flowseal 1.10 parity/test portability patches |

Rust-порт используется потому, что tg-ws работает внутри процесса ZUI, без отдельного Python runtime и отдельного окна. Upstream 2.2.4 содержит порт основной сетевой логики Flowseal вплоть до ветки 1.9.x; применимые изменения оригинала 1.10.0 дополнительно проверены и закрыты локальной версией 2.2.5-zui.1. Python-only GUI/packaging/certifi к ZUI неприменимы.

### Аудит Flowseal 1.9.1 → 1.10.0

| Изменение оригинала | Состояние в ZUI/Rust |
| --- | --- |
| Остановка перебора Worker после успешного connect | `Route::cf_worker` сразу возвращает успешный stream |
| Строгая проверка DC IPv4 вместо permissive `inet_aton` | Rust `IpAddr::parse` уже отвергает short/malformed адреса; поддержка IPv6 остается безопасным расширением |
| Reassembly fragmented WebSocket messages и лимит размера | `tokio-tungstenite` выполняет RFC fragmentation/reassembly; ZUI parity patch снижает message limit с 64 до 16 MiB как в Flowseal 1.10.0 |
| Обновление default CF domains независимо от Python UI toggle | В ZUI fetch привязан к явному profile flag `tgWsDefaultDomains`; пользовательские domains никогда не стираются UI |
| `certifi` для packaged Python/macOS | Неприменимо: Rust использует `rustls` и `webpki-roots` |
| Python/macOS GUI, localization, PyInstaller и docs | Неприменимо к Svelte/Tauri UI; пользовательская настройка fronting добавлена непосредственно в ZUI |
| Новый Python test suite | Rust engine имеет более широкий набор: 188 cases, включая splitter, FakeTLS, pools, outbound, Worker framing, test DC и fronting |

## Что перенесено в Zapret 2

- Официальные x64 `winws2.exe`, `WinDivert.dll`, `WinDivert64.sys`, `cygwin1.dll` из release archive `v1.0.4`.
- Все актуальные upstream Lua-файлы поверх bundled runtime.
- Все актуальные protocol fake payloads; пользовательские payloads набора пресетов сохранены.
- Все официальные Windows helper binaries (`ip2net`, `killall`, `mdig`) и WinDivert filter examples; Discord filter включает UDP `19294-19344`.
- 70 существующих пресетов pack `2.11` сохранены; добавлены 12 проверенных статических профилей `klondike0x/zapret2-youtube-discord` `v1.0.6` (итого 82).
- `manifest.json` содержит tag, commit и hashes; backend проверяет engine version и Lua compatibility marker перед запуском.

Контрольные SHA-256:

| Файл | SHA-256 |
| --- | --- |
| Официальный archive | `5760b6d41c09459fff00b4a6fec5437a471a00aac15f734723ede149cd26c709` |
| `winws2.exe` | `31702b09b424893be881116f3fe9257721d9795e8c570aa27c66a900fe384bfb` |
| `WinDivert.dll` | `16abd6a029e65557c6a309bea7b13bf81fff4e193567582e1cddbf6719f323e0` |
| `WinDivert64.sys` | `8da085332782708d8767bcace5327a6ec7283c17cfb85e40b03cd2323a90ddc2` |
| `cygwin1.dll` | `103104a52e5293ce418944725df19e2bf81ad9269b9a120d71d39028e821499b` |

Автоматическая сверка охватывает 65 официальных файлов из archive: 46 fake payloads, 6 Lua modules, 7 Windows x64 binaries и 6 filter/example files. Все 65 совпадают с archive по SHA-256. Вместе с ресурсами обоих preset pack каталог содержит 106 `.bin`, 12 Lua, 9 executable/DLL/SYS, 21 filter files и 82 пресета.

## Обновление preset pack 2026-08-15

- Classic Flowseal обновлён с `1.9.9a` до `1.10.1` (`47da17f80ad36a8424cdd25658153fdebd7eb938`), release ZIP SHA-256 `f748d61fec75e4edc992cb5b09d554e914197c68c690384aceb61f143d8f76c9`.
- Все 50 файлов Flowseal release archive совпали после копирования; пользовательские `list-general-user.txt`, `list-exclude-user.txt`, `ipset-exclude-user.txt` сохранены отдельно.
- Zapret 2 дополнен 12 профилями `klondike0x/zapret2-youtube-discord v1.0.6` (`f97a4b069f265d813aa83a304786f1ff989eab3f`), release ZIP SHA-256 `c2547c349dc60be8b99e3d5835037ef1d60b4b48b497e7e891f07f7567afdb23`.
- Подробный отбор и причины отказа от непереносимых/непроверяемых источников находятся в `PRESET_RESEARCH_2026-08-15.md`.

## Что перенесено в tg-ws

- Актуальная fallback-цепочка direct WebSocket / Cloudflare proxy / Cloudflare Worker / upstream MTProto / TCP.
- Раздельные TCP/TLS timeout и cooldown, длительный cooldown недоступных DC IP, fronting window.
- Cloudflare proxy/Worker pools, несколько Worker domains, priority и balance.
- Test DC mapping и актуальные DC routing rules.
- Полный desktop settings contract Flowseal: host/port/secret, DC→IP, verbose, ограничение/ротация лога, реальные socket buffers, WS pool, CF Proxy auto/custom, Worker и force test DC.
- Встроенные GUI-тесты CF Proxy и Worker проверяют DC 1/2/3/4/5/203; Worker probe после HTTP 101 отправляет настоящий MTProto init и отбрасывает ложный успех нерабочего TCP tunnel.
- Исправление Worker tunnel framing для больших media uploads: поток не режется по MTProto packet boundaries и не создает WebSocket frames больше лимита Cloudflare.
- Общий outbound connector с HTTP/SOCKS5/SOCKS5h и NO_PROXY, хотя ZUI пока использует direct mode.
- Новая логика лимитов соединений, shared runtime и расширенные проверки.
- В ZUI добавлен profile/UI параметр domain fronting; default `sprinthost.ru` соответствует поведению оригинального Flowseal proxy, пустое значение отключает fronting.
- Сохранено правило ZUI для локальной Telegram-ссылки: явно заданный loopback не подменяется LAN-адресом; wildcard listener рекламирует `127.0.0.1`.

## Граница интеграции

Upstream crate хранится изолированно в `crates/tg-ws-proxy-rs`. ZUI-специфичная конфигурация находится в `src-tauri/src/runtime/tg_ws.rs`. При следующем обновлении crate заменяется новым tagged snapshot, затем адаптер компилируется и проверяется отдельно. Не переносить Python GUI, embedded Python или отдельный tg-ws executable: это нарушит текущую in-process архитектуру ZUI.

Локальные отличия от tagged `v2.2.4`: runtime ограничивает reassembled incoming WebSocket message до 16 MiB для parity с Flowseal 1.10.0; `buf_kb` реально применяется к inbound/outbound TCP sockets; реализован Flowseal force-test-DC маршрут с test IP и `/apiws_test`; библиотечный CF checker возвращает структурированные результаты всех DC; два `api.localhost` NO_PROXY cases помечены ignored на Windows, где системный resolver не поддерживает subdomain localhost; fake HTTP proxy в одном WS integration test принимает обе ожидаемые попытки, чтобы закрытие первого socket не превращалось в Windows TCP timeout. Поэтому встроенный crate имеет отдельную версию `2.2.5-zui.1`.
