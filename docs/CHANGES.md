# Changes

## 1.5.0

- Переработана страница Home: спокойные карточки сервисов, анимированный glow, блок готовности и более чистая зона запуска.
- Таблица пресетов переведена с `QTableWidget` на `QTableView` + `QAbstractTableModel` для лучшей производительности и более плавного скролла.
- Добавлен debounce для поиска и фильтров пресетов, чтобы уменьшить лишние обновления интерфейса.
- Логи Activity теперь буферизуются, чтобы не перегружать интерфейс частыми перерисовками во время тестов и вывода сервисов.
- Добавлен встроенный onboarding-тур с правой панелью подсказок и контекстной подсветкой.
- Добавлен in-app busy overlay с анимированным прогрессом для долгих операций вместо ожидания только через системный курсор.
- Запуск и остановка сервисов частично вынесены в worker-потоки, чтобы меньше блокировать GUI-поток.
- Встроенная source-интеграция `tg-ws-proxy` обновлена до upstream `v1.7.0`.
- Улучшен Fluent-вид таблицы и элементов управления на Home.
- Исправлены проблемы с кодировкой русского текста в новых строках интерфейса.
- Исправлена работа чекбоксов выбора тестов в таблице пресетов.
- Исправлен краш окна настроек теста из-за `stateChanged`, который передавал числовое состояние чекбокса.
- Исправлены цвета карточек сервисов Home в светлой теме.
- Диалог подтверждения закрытия заменён на кастомный Fluent-диалог.
- Добавлены crash reports и диагностика запуска, чтобы проще ловить тихие падения приложения.

## 1.0.1

- Faster preset testing: targets are checked in parallel while keeping one active `winws2` preset at a time.
- Added update checking from GitHub Releases.
- Added `CHANGES.md` for in-app release notes.

## 1.0.0

- First public portable Windows release.
- Automatic zapret2 preset testing.
- Favorite presets and saved test results.
- One-click launch from the Home page.
- Built-in tg-ws-proxy controls.
- VPN/proxy conflict detection before launch.
- English and Russian interface languages.
- Windows 11-inspired UI with light, dark, and AMOLED themes.
