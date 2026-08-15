# Аудит актуальных пресетов Zapret 2 и Classic

Дата проверки: 2026-08-15. Это рабочий аудит исходников и локального набора ZUI, а не релизный документ.

## Итог

- Движок `bol-van/zapret2` уже актуален: последний стабильный релиз — `v1.0.4`. Менять `winws2.exe` не потребовалось.
- К существующим 70 Zapret 2-пресетам добавлены 12 готовых статических профилей из `klondike0x/zapret2-youtube-discord v1.0.6`. Они имеют префикс `KLD 1.0.6 -`, чтобы не подменять старые варианты с похожими именами.
- Zapret 2 теперь содержит 82 пресета и 106 payload-файлов. Для всех 82 конфигураций проверено наличие всех 111 ссылочных ресурсов.
- Classic Flowseal заменён с `1.9.9a` на стабильный `1.10.1`: 21 запускаемый preset, включая новый `general (EXP)`, новые UDP/payload-файлы и обновлённые списки.
- Три пользовательских Flowseal-файла сохранены при обновлении: `list-general-user.txt`, `list-exclude-user.txt`, `ipset-exclude-user.txt`.
- Старый локальный каталог Flowseal сохранён для отката в `data/upstream-backups/flowseal-1.9.9a` и больше не участвует в обнаружении пресетов.

## Проверенные первичные источники

| Источник | Состояние на дату аудита | Решение |
| --- | --- | --- |
| [bol-van/zapret2](https://github.com/bol-van/zapret2) | Последний release `v1.0.4`, Lua compat 6 | Текущий движок оставить |
| [Flowseal/zapret-discord-youtube](https://github.com/Flowseal/zapret-discord-youtube) | Stable `1.10.1`, опубликован 2026-08-09 | Полностью синхронизировать Classic pack |
| [klondike0x/zapret2-youtube-discord](https://github.com/klondike0x/zapret2-youtube-discord) | `v1.0.6`, 12 готовых `.txt`, MIT, release checksums/GPG | Добавить все 12 как отдельную группу |
| [Asterlike/zapret2UI](https://github.com/Asterlike/zapret2UI) | `1.0.0`, 9 встроенных стратегий создаются C#-кодом и используют runtime-токены | Не копировать вслепую; нужен отдельный конвертер |
| [ArcheKaren/Traffic-Profile-Manager](https://github.com/ArcheKaren/Traffic-Profile-Manager) | `1.2.0`, 14 JSON-профилей с rule groups, mappings и runtime-компоновкой | Не импортировать как `.txt` без переноса компоновщика |
| [Dunterbabochka/zapret2-next](https://github.com/Dunterbabochka/zapret2-next) | `v0.2.0`, стратегии — шаблоны `.txt.in`, комплект основан на более старом engine `v1.0.2` | Не добавлять как готовые конфиги |
| `kinlay0/zapret2-discord-youtube-strats` | Свежие TEST-файлы, но нет лицензии; README прямо предупреждает о нестабильности | Исключить из поставки |
| `CloudLoopOven/Zapret-for-Telegram` | Репозиторий содержит описание/картинки, но не проверяемый исходный код заявленного бинарника | Исключить из поставки |

Старый адрес `youtubediscord/zapret`, с которым ассоциирован legacy pack `2.11`, на дату аудита отвечает `404` через GitHub API. Поэтому 70 существующих пресетов сохранены без переприсвоения происхождения, а новый проверяемый источник записан отдельно в manifest.

## Provenance и целостность

### Zapret 2 additions

- Tag: `v1.0.6`
- Commit: `f97a4b069f265d813aa83a304786f1ff989eab3f`
- Release archive SHA-256: `c2547c349dc60be8b99e3d5835037ef1d60b4b48b497e7e891f07f7567afdb23`
- Добавлено: 12 профилей, `ACTIVE_DISCORD_UDP.bin`, `ACTIVE_GAME_UDP.bin` и три обязательных user-list placeholder.
- Содержимое профилей скопировано байт-в-байт; изменены только имена файлов в ZUI.

### Flowseal Classic

- Tag: `1.10.1`
- Commit: `47da17f80ad36a8424cdd25658153fdebd7eb938`
- Release archive SHA-256: `f748d61fec75e4edc992cb5b09d554e914197c68c690384aceb61f143d8f76c9`
- Все 50 файлов release archive после копирования совпали с источником по SHA-256.
- Дополнительно перенесены три пользовательских файла из прежнего локального pack.

## Что ещё требует ручной проверки

`winws2.exe` имеет Windows-манифест `requireAdministrator`; автоматический `--dry-run` из текущей не elevated-сессии Windows был заблокирован UAC. Поэтому перед релизом вручную:

1. Запустить минимум `KLD 1.0.6 - General`, `General ALT`, `General HostFakeSplit`, `Discord`, `YouTube` и `Rostelecom`.
2. Для каждого проверить отсутствие ошибок Lua/blob/list, YouTube, Discord text/media/voice и остановку процесса.
3. Прогнать `Flowseal 1.10.1` как минимум `general`, `ALT3`, `ALT11`, `ALT12`, `EXP` и один `FAKE TLS AUTO`.
4. Затем запустить встроенный тест всех/выбранных пресетов на реальной сети пользователя.

Повторяемая статическая проверка: `powershell -ExecutionPolicy Bypass -File tools/verify-preset-bundle.ps1`.
