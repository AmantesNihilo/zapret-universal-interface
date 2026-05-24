from __future__ import annotations



import os

import platform

import re

import subprocess

import sys

import time

import ctypes

import zipfile

from pathlib import Path



from PySide6.QtCore import QEasingCurve, QEvent, QPoint, QRect, QRectF, QPropertyAnimation, QSize, QThread, QTimer, Qt, QUrl

from PySide6.QtGui import QAction, QColor, QCursor, QDesktopServices, QIcon, QImage, QPainter, QPalette, QPen, QPixmap

from PySide6.QtWidgets import (

    QAbstractItemView,

    QApplication,

    QButtonGroup,

    QCheckBox,

    QDialog,

    QFileDialog,

    QFrame,

    QGridLayout,

    QGraphicsOpacityEffect,

    QHBoxLayout,

    QHeaderView,

    QMenu,

    QMessageBox,

    QPushButton as QtPushButton,

    QScrollArea,

    QSpinBox,

    QStackedWidget,

    QSystemTrayIcon,

    QTableView,

    QVBoxLayout,

    QWidget,

)

from qfluentwidgets import (

    FluentIcon,

    InfoBar,

    InfoBarPosition,

    LineEdit,

    MessageBox,

    MSFluentWindow,

    NavigationItemPosition,

    SwitchButton,

    Theme,

    ToolButton,

    setTheme,

    setThemeColor,

)



from zapret_core import admin, conflicts, presets, results as stored_results, runner, scoring, settings, state, targets, tg_ws

from zapret_core.updater import UpdateInfo

from zapret_core.paths import AppPaths

from ui.conflict_dialog import ConflictDialog

from ui.fluent_widgets import (

    BodyLabel as QLabel,

    ComboBox as FluentComboBox,

    GlowServiceCard,

    PrimaryButton,

    ProgressBar as QProgressBar,

    PushButton as QPushButton,

    SearchLineEdit as QLineEdit,

    SORT_VALUE_ROLE,

    TextEdit as QTextEdit,

    panel_card,

    setting_row,

)

from ui.preset_table import PresetFilterProxyModel, PresetTableModel, PresetTableRow

from ui.style import app_qss

from ui.test_settings_dialog import TestSettingsDialog

from ui.workers import ServiceOperationWorker, TestAllWorker, UpdateCheckWorker





THEME_OPTIONS = [("Windows 11", "win11"), ("Dark", "dark"), ("AMOLED", "amoled"), ("Light", "light")]

LANGUAGE_OPTIONS = [("English", "en"), ("\u0420\u0443\u0441\u0441\u043a\u0438\u0439", "ru")]

APP_VERSION = "1.5.0"

PROJECT_RELEASES_URL = "https://github.com/AmantesNihilo/Z2-GUI-windows/releases"

CLOSE_OPTIONS = [

    ("Ask every time", "ask"),

    ("Stop winws2 and close", "stop"),

    ("Leave winws2 running", "keep"),

]



RU_TEXT = {

    'Home': 'Главная',

    'Presets': 'Пресеты',

    'TG': 'TG',

    'Activity': 'Актив',

    'Settings': 'Настройки',

    'Testing': 'Тестирование',

    'Current Run': 'Текущий тест',

    'Actions': 'Действия',

    'Before Testing': 'Перед тестом',

    'Run a short check to find working configs.': 'Запустите короткую проверку, чтобы найти рабочие конфиги.',

    'Star configs you want to launch from Home.': 'Добавьте в избранное конфиги, которые хотите запускать с главной.',

    'Choose a favorite preset and launch it.': 'Выберите избранный пресет и запустите его.',

    'Quick setup': 'Быстрая настройка',

    'Test presets': 'Проверить пресеты',

    'Pick favorites': 'Выбрать избранное',

    'Start': 'Старт',

    'Stop': 'Стоп',

    'Stop winws2': 'Остановить winws2',

    'Show Z2 GUI': 'Показать Z2 GUI',

    'Close': 'Закрыть',

    'Idle': 'Ожидание',

    'Stop test': 'Остановить тест',

    'Cancel Test': 'Отменить тест',

    'Stopping...': 'Остановка...',

    'Selected preset': 'Выбранный пресет',

    'Test Selected': 'Проверить выбранные',

    'Test Settings': 'Настройки теста',

    'Test Settings: All': 'Настройки теста: все',

    'Profile: All targets': 'Профиль: все цели',

    'Pick Test Targets': 'Выбрать цели теста',

    'Open Favorites': 'Открыть избранное',

    'Run Preset': 'Запустить пресет',

    'Add Preset': 'Добавить пресет',

    'Open Presets Folder': 'Открыть папку пресетов',

    'Delete/Hide': 'Удалить/скрыть',

    'Restore Hidden': 'Вернуть скрытые',

    'Select All': 'Выбрать все',

    'Select Fav': 'Выбрать избранное',

    'Clear': 'Очистить',

    'Refresh': 'Обновить',

    'Restart as Admin': 'Перезапустить от админа',

    'No preset selected': 'Пресет не выбран',

    'Status: -': 'Статус: -',

    'Score: -': 'Оценка: -',

    'Checks: -': 'Проверки: -',

    'Ping: -': 'Пинг: -',

    'Path: -': 'Путь: -',

    'File: -': 'Файл: -',

    'State: -': 'Состояние: -',

    'Active preset: -': 'Активный пресет: -',

    'Telegram proxy integration is not enabled yet': 'Интеграция Telegram proxy еще не включена',

    'Start with main button': 'Запускать главной кнопкой',

    'Presets: -': 'Пресетов: -',

    'Favorites: -': 'Избранных: -',

    'Tested: -': 'Проверено: -',

    'Best preset: -': 'Лучший пресет: -',

    'Targets: -': 'Цели: -',

    'Targets': 'Цели',

    'Active preset': 'Активный пресет',

    'Best preset': 'Лучший пресет',

    'Selected presets': 'Выбранных пресетов',

    'Best result': 'Лучший результат',

    'Tested': 'Проверено',

    'Root': 'Корень',

    'Selected preset: -': 'Выбранный пресет: -',

    'Proxy link: -': 'Proxy-ссылка: -',

    'Proxy link': 'Proxy-ссылка',

    'winws2: -': 'winws2: -',

    'Root: -': 'Корень: -',

    'Search presets, status, path...': 'Поиск пресетов, статуса, пути...',

    'Show': 'Показать',

    'All': 'Все',

    'Passed': 'Прошли',

    'Passed + Partial': 'Прошли + частично',

    'Favorites': 'Избранное',

    'Search': 'Поиск',

    'Selection': 'Выбор',

    'None': 'Ничего',

    'Fav': 'Избр.',

    'Test': 'Тест',

    'Preset': 'Пресет',

    'Status': 'Статус',

    'FAIL': 'FAIL',

    'UNSUP': 'UNSUP',

    'Ping': 'Пинг',

    'Score': 'Оценка',

    'Services': '\u0421\u0435\u0440\u0432\u0438\u0441\u044b',

    'Clear Log': 'Очистить лог',

    'TG WS Proxy': 'TG WS Proxy',

    'Proxy Settings': 'Настройки proxy',

    'Port': 'Порт',

    'Secret': 'Секрет',

    'Copy proxy link': 'Копировать proxy-ссылку',

    'Open in Telegram': 'Открыть в Telegram',

    'Restart tg-ws': 'Перезапустить tg-ws',

    'Open log': 'Открыть лог',

    'About': 'О приложении',

    'Appearance': 'Внешний вид',

    'Behavior': 'Поведение',

    'Tools': 'Инструменты',

    'Credits': 'Credits',

    'Theme': 'Тема',

    'Language': 'Язык',

    'On close with winws2': 'При закрытии с winws2',

    'Open Settings JSON': 'Открыть JSON настроек',

    'Open Results JSON': 'Открыть JSON результатов',

    'Show Welcome Guide': 'Показать приветствие',

    'Export Settings': 'Экспорт настроек',

    'Import Settings': 'Импорт настроек',

    'Exports settings, last test results, and custom preset files.': 'Экспортирует настройки, последние результаты тестов и пользовательские пресеты.',

    'Admin': 'Админ',

    'Not admin': 'Не админ',

    'Stopped': 'Остановлено',

    'Running': 'Запущено',

    'Needs admin': 'Нужен админ',

    'Running: tg-ws': 'Запущено: tg-ws',

    'Stopping test...': 'Остановка теста...',

    'Stopping winws2...': 'Остановка winws2...',

    'Stopping test': 'Остановка теста',

    'Waiting for current check to finish.': 'Ждем завершения текущей проверки.',

    'Stopped selected services.': 'Выбранные сервисы остановлены.',

    'Selected services stopped.': 'Выбранные сервисы остановлены.',

    'Stopped tg-ws.': 'tg-ws остановлен.',

    'Select Service': 'Выберите сервис',

    'Add Favorite': 'Добавьте избранное',

    'Enable zapret or tg-ws on the Home cards first': 'Сначала включите zapret или tg-ws на карточках Home',

    'included in Start': 'включено в Start',

    'manual only': 'только вручную',

    'Add presets to Favorites first': 'Сначала добавьте пресеты в избранное',

    'No favorite presets yet. Star one or more presets on the Presets page.': 'Избранных пресетов пока нет. Добавьте один или несколько пресетов звездочкой на странице Presets.',

    'Z2 GUI is a portable Windows utility for testing zapret2 presets, launching the selected working configuration, and managing tg-ws proxy from one clean interface.': 'Z2 GUI — портативная Windows-утилита для проверки zapret2-пресетов, запуска выбранной рабочей конфигурации и управления tg-ws proxy из одного удобного интерфейса.',

    'Updates': '\u041e\u0431\u043d\u043e\u0432\u043b\u0435\u043d\u0438\u044f',

    'Check for Updates': '\u041f\u0440\u043e\u0432\u0435\u0440\u0438\u0442\u044c \u043e\u0431\u043d\u043e\u0432\u043b\u0435\u043d\u0438\u044f',

    'Checking...': '\u041f\u0440\u043e\u0432\u0435\u0440\u043a\u0430...',

    'Open Releases': '\u041e\u0442\u043a\u0440\u044b\u0442\u044c \u0440\u0435\u043b\u0438\u0437\u044b',

    'Z2 GUI checks GitHub Releases and shows CHANGES.md. Export your settings before installing a new portable build.': '\u005a2 GUI \u043f\u0440\u043e\u0432\u0435\u0440\u044f\u0435\u0442 GitHub Releases \u0438 \u043f\u043e\u043a\u0430\u0437\u044b\u0432\u0430\u0435\u0442 CHANGES.md. \u042d\u043a\u0441\u043f\u043e\u0440\u0442\u0438\u0440\u0443\u0439\u0442\u0435 \u043d\u0430\u0441\u0442\u0440\u043e\u0439\u043a\u0438 \u043f\u0435\u0440\u0435\u0434 \u0443\u0441\u0442\u0430\u043d\u043e\u0432\u043a\u043e\u0439 \u043d\u043e\u0432\u043e\u0439 portable-\u0441\u0431\u043e\u0440\u043a\u0438.',

    'Open Release': '\u041e\u0442\u043a\u0440\u044b\u0442\u044c \u0440\u0435\u043b\u0438\u0437',

    'Download Portable': '\u0421\u043a\u0430\u0447\u0430\u0442\u044c portable',

    'Later': '\u041f\u043e\u0437\u0436\u0435',

    'Changes': '\u0418\u0437\u043c\u0435\u043d\u0435\u043d\u0438\u044f',

    'Update Available': '\u0414\u043e\u0441\u0442\u0443\u043f\u043d\u043e \u043e\u0431\u043d\u043e\u0432\u043b\u0435\u043d\u0438\u0435',

    'Test Details': '\u0414\u0435\u0442\u0430\u043b\u0438 \u0442\u0435\u0441\u0442\u0430',

}




RU_TEXT.update({
    "Launch": "\u0417\u0430\u043f\u0443\u0441\u043a",
    "Readiness": "\u0413\u043e\u0442\u043e\u0432\u043d\u043e\u0441\u0442\u044c",
    "Administrator": "\u0410\u0434\u043c\u0438\u043d\u0438\u0441\u0442\u0440\u0430\u0442\u043e\u0440",
    "Favorite preset": "\u0418\u0437\u0431\u0440\u0430\u043d\u043d\u044b\u0439 \u043f\u0440\u0435\u0441\u0435\u0442",
    "Test targets": "\u0426\u0435\u043b\u0438 \u0442\u0435\u0441\u0442\u0430",
    "Launch services": "\u0421\u0435\u0440\u0432\u0438\u0441\u044b \u0437\u0430\u043f\u0443\u0441\u043a\u0430",
    "Ready": "\u0413\u043e\u0442\u043e\u0432\u043e",
    "Run as administrator to start zapret": "\u0417\u0430\u043f\u0443\u0441\u0442\u0438\u0442\u0435 \u043e\u0442 \u0430\u0434\u043c\u0438\u043d\u0438\u0441\u0442\u0440\u0430\u0442\u043e\u0440\u0430 \u0434\u043b\u044f \u0437\u0430\u043f\u0443\u0441\u043a\u0430 zapret",
    "Choose a favorite preset": "\u0412\u044b\u0431\u0435\u0440\u0438\u0442\u0435 \u0438\u0437\u0431\u0440\u0430\u043d\u043d\u044b\u0439 \u043f\u0440\u0435\u0441\u0435\u0442",
    "Add a preset to Favorites": "\u0414\u043e\u0431\u0430\u0432\u044c\u0442\u0435 \u043f\u0440\u0435\u0441\u0435\u0442 \u0432 \u0438\u0437\u0431\u0440\u0430\u043d\u043d\u043e\u0435",
    "Select at least one service": "\u0412\u044b\u0431\u0435\u0440\u0438\u0442\u0435 \u0445\u043e\u0442\u044f \u0431\u044b \u043e\u0434\u0438\u043d \u0441\u0435\u0440\u0432\u0438\u0441",
    "Welcome to Z2 GUI": "\u0414\u043e\u0431\u0440\u043e \u043f\u043e\u0436\u0430\u043b\u043e\u0432\u0430\u0442\u044c \u0432 Z2 GUI",
    "Administrator rights": "\u041f\u0440\u0430\u0432\u0430 \u0430\u0434\u043c\u0438\u043d\u0438\u0441\u0442\u0440\u0430\u0442\u043e\u0440\u0430",
    "Presets page": "\u0421\u0442\u0440\u0430\u043d\u0438\u0446\u0430 \u043f\u0440\u0435\u0441\u0435\u0442\u043e\u0432",
    "Test settings": "\u041d\u0430\u0441\u0442\u0440\u043e\u0439\u043a\u0438 \u0442\u0435\u0441\u0442\u0430",
    "Results and score": "\u0420\u0435\u0437\u0443\u043b\u044c\u0442\u0430\u0442\u044b \u0438 \u043e\u0446\u0435\u043d\u043a\u0430",
    "Home launch": "\u0417\u0430\u043f\u0443\u0441\u043a \u043d\u0430 Home",
    "tg-ws proxy": "\u041f\u0440\u043e\u043a\u0441\u0438 tg-ws",
    "Activity logs": "\u0416\u0443\u0440\u043d\u0430\u043b Activity",
    "You are ready": "\u0412\u0441\u0435 \u0433\u043e\u0442\u043e\u0432\u043e",
    "Skip": "\u041f\u0440\u043e\u043f\u0443\u0441\u0442\u0438\u0442\u044c",
    "Back": "\u041d\u0430\u0437\u0430\u0434",
    "Next": "\u0414\u0430\u043b\u0435\u0435",
    "Finish": "\u0413\u043e\u0442\u043e\u0432\u043e",
    "Tour route": "\u041c\u0430\u0440\u0448\u0440\u0443\u0442",
    "Please wait": "\u041f\u043e\u0434\u043e\u0436\u0434\u0438\u0442\u0435",
    "winws2 is still running.": "winws2 \u0432\u0441\u0435 \u0435\u0449\u0435 \u0440\u0430\u0431\u043e\u0442\u0430\u0435\u0442.",
    "Stop winws2 before closing the app?": "\u041e\u0441\u0442\u0430\u043d\u043e\u0432\u0438\u0442\u044c winws2 \u043f\u0435\u0440\u0435\u0434 \u0437\u0430\u043a\u0440\u044b\u0442\u0438\u0435\u043c?",
    "Yes, stop it": "\u0414\u0430, \u043e\u0441\u0442\u0430\u043d\u043e\u0432\u0438\u0442\u044c",
    "No, leave running": "\u041d\u0435\u0442, \u043e\u0441\u0442\u0430\u0432\u0438\u0442\u044c",
    "Cancel": "\u041e\u0442\u043c\u0435\u043d\u0430",
    "Don't ask again": "\u0411\u043e\u043b\u044c\u0448\u0435 \u043d\u0435 \u0441\u043f\u0440\u0430\u0448\u0438\u0432\u0430\u0442\u044c",
    "You can replay this guide later from Settings / Tools.": "\u041f\u043e\u0432\u0442\u043e\u0440\u0438\u0442\u044c \u043e\u0431\u0443\u0447\u0435\u043d\u0438\u0435 \u043c\u043e\u0436\u043d\u043e \u043f\u043e\u0437\u0436\u0435 \u0432 Settings / Tools.",
    "Z2 GUI helps you test zapret presets, save working ones, launch them from Home, and manage the optional tg-ws Telegram proxy. It is not a VPN.": "Z2 GUI \u043f\u043e\u043c\u043e\u0433\u0430\u0435\u0442 \u0442\u0435\u0441\u0442\u0438\u0440\u043e\u0432\u0430\u0442\u044c zapret-\u043f\u0440\u0435\u0441\u0435\u0442\u044b, \u0441\u043e\u0445\u0440\u0430\u043d\u044f\u0442\u044c \u0440\u0430\u0431\u043e\u0447\u0438\u0435 \u0432\u0430\u0440\u0438\u0430\u043d\u0442\u044b, \u0437\u0430\u043f\u0443\u0441\u043a\u0430\u0442\u044c \u0438\u0445 \u0441 Home \u0438 \u0443\u043f\u0440\u0430\u0432\u043b\u044f\u0442\u044c \u043e\u043f\u0446\u0438\u043e\u043d\u0430\u043b\u044c\u043d\u044b\u043c Telegram-\u043f\u0440\u043e\u043a\u0441\u0438 tg-ws. \u042d\u0442\u043e \u043d\u0435 VPN.",
    "Starting zapret requires administrator rights because winws2 and WinDivert work with network packets. Browsing presets and configuring tg-ws does not require elevation.": "\u0414\u043b\u044f \u0437\u0430\u043f\u0443\u0441\u043a\u0430 zapret \u043d\u0443\u0436\u043d\u044b \u043f\u0440\u0430\u0432\u0430 \u0430\u0434\u043c\u0438\u043d\u0438\u0441\u0442\u0440\u0430\u0442\u043e\u0440\u0430, \u043f\u043e\u0442\u043e\u043c\u0443 \u0447\u0442\u043e winws2 \u0438 WinDivert \u0440\u0430\u0431\u043e\u0442\u0430\u044e\u0442 \u0441 \u0441\u0435\u0442\u0435\u0432\u044b\u043c\u0438 \u043f\u0430\u043a\u0435\u0442\u0430\u043c\u0438. \u041f\u0440\u043e\u0441\u043c\u043e\u0442\u0440 \u043f\u0440\u0435\u0441\u0435\u0442\u043e\u0432 \u0438 \u043d\u0430\u0441\u0442\u0440\u043e\u0439\u043a\u0430 tg-ws \u043d\u0435 \u0442\u0440\u0435\u0431\u0443\u044e\u0442 \u043f\u043e\u0432\u044b\u0448\u0435\u043d\u0438\u044f \u043f\u0440\u0430\u0432.",
    "The Presets page contains all available configurations. Use the Test column to choose what to check, and use the star column to add good presets to Favorites.": "\u041d\u0430 \u0441\u0442\u0440\u0430\u043d\u0438\u0446\u0435 Presets \u0441\u043e\u0431\u0440\u0430\u043d\u044b \u0432\u0441\u0435 \u0434\u043e\u0441\u0442\u0443\u043f\u043d\u044b\u0435 \u043a\u043e\u043d\u0444\u0438\u0433\u0443\u0440\u0430\u0446\u0438\u0438. \u0412 \u043a\u043e\u043b\u043e\u043d\u043a\u0435 Test \u0432\u044b\u0431\u0438\u0440\u0430\u0439\u0442\u0435, \u0447\u0442\u043e \u043f\u0440\u043e\u0432\u0435\u0440\u044f\u0442\u044c, \u0430 \u0437\u0432\u0435\u0437\u0434\u043e\u0439 \u0434\u043e\u0431\u0430\u0432\u043b\u044f\u0439\u0442\u0435 \u0443\u0434\u0430\u0447\u043d\u044b\u0435 \u043f\u0440\u0435\u0441\u0435\u0442\u044b \u0432 \u0438\u0437\u0431\u0440\u0430\u043d\u043d\u043e\u0435.",
    "Open Test Settings to choose services and endpoints. Fewer targets make checks faster when you only need Discord, YouTube, Telegram, or another specific service.": "\u041e\u0442\u043a\u0440\u043e\u0439\u0442\u0435 Test Settings, \u0447\u0442\u043e\u0431\u044b \u0432\u044b\u0431\u0440\u0430\u0442\u044c \u0441\u0435\u0440\u0432\u0438\u0441\u044b \u0438 endpoints. \u0427\u0435\u043c \u043c\u0435\u043d\u044c\u0448\u0435 \u0446\u0435\u043b\u0435\u0439, \u0442\u0435\u043c \u0431\u044b\u0441\u0442\u0440\u0435\u0435 \u0442\u0435\u0441\u0442, \u043a\u043e\u0433\u0434\u0430 \u043d\u0443\u0436\u0435\u043d \u0442\u043e\u043b\u044c\u043a\u043e Discord, YouTube, Telegram \u0438\u043b\u0438 \u0434\u0440\u0443\u0433\u043e\u0439 \u043a\u043e\u043d\u043a\u0440\u0435\u0442\u043d\u044b\u0439 \u0441\u0435\u0440\u0432\u0438\u0441.",
    "After testing, look at Status, Services, Ping, and Score. Higher score usually means a better candidate, but real network behavior can still vary.": "\u041f\u043e\u0441\u043b\u0435 \u0442\u0435\u0441\u0442\u0430 \u0441\u043c\u043e\u0442\u0440\u0438\u0442\u0435 \u043d\u0430 Status, Services, Ping \u0438 Score. \u0411\u043e\u043b\u0435\u0435 \u0432\u044b\u0441\u043e\u043a\u0438\u0439 score \u043e\u0431\u044b\u0447\u043d\u043e \u043e\u0437\u043d\u0430\u0447\u0430\u0435\u0442 \u043b\u0443\u0447\u0448\u0438\u0439 \u0432\u0430\u0440\u0438\u0430\u043d\u0442, \u043d\u043e \u0440\u0435\u0430\u043b\u044c\u043d\u0430\u044f \u0441\u0435\u0442\u044c \u0432\u0441\u0435 \u0435\u0449\u0435 \u043c\u043e\u0436\u0435\u0442 \u0432\u0435\u0441\u0442\u0438 \u0441\u0435\u0431\u044f \u043f\u043e-\u0440\u0430\u0437\u043d\u043e\u043c\u0443.",
    "Home is the control panel. Choose a favorite preset, enable zapret and/or tg-ws with the switches, then press Start.": "Home - \u044d\u0442\u043e \u043f\u0430\u043d\u0435\u043b\u044c \u0437\u0430\u043f\u0443\u0441\u043a\u0430. \u0412\u044b\u0431\u0435\u0440\u0438\u0442\u0435 \u0438\u0437\u0431\u0440\u0430\u043d\u043d\u044b\u0439 \u043f\u0440\u0435\u0441\u0435\u0442, \u0432\u043a\u043b\u044e\u0447\u0438\u0442\u0435 zapret \u0438/\u0438\u043b\u0438 tg-ws \u043f\u0435\u0440\u0435\u043a\u043b\u044e\u0447\u0430\u0442\u0435\u043b\u044f\u043c\u0438, \u0437\u0430\u0442\u0435\u043c \u043d\u0430\u0436\u043c\u0438\u0442\u0435 Start.",
    "The TG page manages the local Telegram MTProto WebSocket proxy. You can set the port and secret, restart it, copy the link, or open it in Telegram.": "\u0421\u0442\u0440\u0430\u043d\u0438\u0446\u0430 TG \u0443\u043f\u0440\u0430\u0432\u043b\u044f\u0435\u0442 \u043b\u043e\u043a\u0430\u043b\u044c\u043d\u044b\u043c Telegram MTProto WebSocket proxy. \u041c\u043e\u0436\u043d\u043e \u0437\u0430\u0434\u0430\u0442\u044c \u043f\u043e\u0440\u0442 \u0438 secret, \u043f\u0435\u0440\u0435\u0437\u0430\u043f\u0443\u0441\u0442\u0438\u0442\u044c \u0435\u0433\u043e, \u0441\u043a\u043e\u043f\u0438\u0440\u043e\u0432\u0430\u0442\u044c \u0441\u0441\u044b\u043b\u043a\u0443 \u0438\u043b\u0438 \u043e\u0442\u043a\u0440\u044b\u0442\u044c \u0435\u0435 \u0432 Telegram.",
    "Activity shows separate logs for zapret and tg-ws. When something fails, this page is the first place to inspect what happened.": "Activity \u043f\u043e\u043a\u0430\u0437\u044b\u0432\u0430\u0435\u0442 \u043e\u0442\u0434\u0435\u043b\u044c\u043d\u044b\u0435 \u043b\u043e\u0433\u0438 zapret \u0438 tg-ws. \u0415\u0441\u043b\u0438 \u0447\u0442\u043e-\u0442\u043e \u043d\u0435 \u0437\u0430\u043f\u0443\u0441\u0442\u0438\u043b\u043e\u0441\u044c, \u0441\u043d\u0430\u0447\u0430\u043b\u0430 \u0441\u043c\u043e\u0442\u0440\u0438\u0442\u0435 \u0441\u044e\u0434\u0430.",
    "A good first run is: open Presets, select a few configs, test them, star the best result, return Home, and press Start.": "\u0425\u043e\u0440\u043e\u0448\u0438\u0439 \u043f\u0435\u0440\u0432\u044b\u0439 \u0437\u0430\u043f\u0443\u0441\u043a: \u043e\u0442\u043a\u0440\u043e\u0439\u0442\u0435 Presets, \u0432\u044b\u0431\u0435\u0440\u0438\u0442\u0435 \u043d\u0435\u0441\u043a\u043e\u043b\u044c\u043a\u043e \u043a\u043e\u043d\u0444\u0438\u0433\u043e\u0432, \u043f\u0440\u043e\u0442\u0435\u0441\u0442\u0438\u0440\u0443\u0439\u0442\u0435 \u0438\u0445, \u0434\u043e\u0431\u0430\u0432\u044c\u0442\u0435 \u043b\u0443\u0447\u0448\u0438\u0439 \u0432 \u0438\u0437\u0431\u0440\u0430\u043d\u043d\u043e\u0435, \u0432\u0435\u0440\u043d\u0438\u0442\u0435\u0441\u044c \u043d\u0430 Home \u0438 \u043d\u0430\u0436\u043c\u0438\u0442\u0435 Start.",
})


class BusyProgressLine(QWidget):

    def __init__(self, parent: QWidget | None = None) -> None:

        super().__init__(parent)

        self._phase = 0

        self.setObjectName("BusyProgressLine")

        self.setMinimumHeight(10)

        self.setMaximumHeight(10)

    def set_phase(self, phase: int) -> None:

        self._phase = phase

        self.update()

    def paintEvent(self, event) -> None:

        painter = QPainter(self)

        painter.setRenderHint(QPainter.Antialiasing, True)

        rect = QRectF(0, self.height() / 2 - 1, self.width(), 2)

        base_pen = QPen(QColor(135, 145, 160, 52), 2)

        base_pen.setCapStyle(Qt.RoundCap)

        painter.setPen(base_pen)

        painter.drawLine(rect.left(), rect.center().y(), rect.right(), rect.center().y())

        if self.width() <= 0:

            return

        segment = max(64, int(self.width() * 0.22))

        travel = self.width() + segment * 2

        x = ((self._phase * 7) % travel) - segment

        glow_pen = QPen(QColor(64, 156, 255, 34), 5)

        glow_pen.setCapStyle(Qt.RoundCap)

        painter.setPen(glow_pen)

        painter.drawLine(x, rect.center().y(), x + segment, rect.center().y())

        line_pen = QPen(QColor(96, 174, 255), 2)

        line_pen.setCapStyle(Qt.RoundCap)

        painter.setPen(line_pen)

        painter.drawLine(x, rect.center().y(), x + segment, rect.center().y())


class BusyOverlay(QWidget):

    def __init__(self, owner: "MainWindow") -> None:

        super().__init__(owner)

        self.owner = owner

        self._phase = 0

        self._base_message = ""

        self.setObjectName("BusyOverlay")

        self.setAttribute(Qt.WA_StyledBackground, False)

        self.setFocusPolicy(Qt.NoFocus)

        self.hide()

        self.owner.installEventFilter(self)

        self.panel = QFrame(self)

        self.panel.setObjectName("BusyPanel")

        self.panel.setFixedWidth(328)

        layout = QVBoxLayout(self.panel)

        layout.setContentsMargins(24, 22, 24, 22)

        layout.setSpacing(14)

        text_box = QVBoxLayout()

        text_box.setContentsMargins(0, 0, 0, 0)

        text_box.setSpacing(6)

        self.title_label = QLabel(owner.tr("Please wait"))

        self.title_label.setObjectName("BusyTitle")

        self.message_label = QLabel()

        self.message_label.setObjectName("MetaLabel")

        self.message_label.setWordWrap(True)

        text_box.addWidget(self.title_label)

        text_box.addWidget(self.message_label)

        layout.addLayout(text_box)

        self.progress = BusyProgressLine(self.panel)

        layout.addWidget(self.progress)

        self.timer = QTimer(self)

        self.timer.setInterval(33)

        self.timer.timeout.connect(self.tick)

    def eventFilter(self, watched, event) -> bool:

        if watched is self.owner and event.type() in (QEvent.Resize, QEvent.Move):

            self.setGeometry(self.owner.rect())

            self.position_panel()

        return super().eventFilter(watched, event)

    def paintEvent(self, event) -> None:

        painter = QPainter(self)

        painter.fillRect(self.rect(), QColor(0, 0, 0, 66))

    def set_message(self, text: str) -> None:

        self.title_label.setText(self.owner.tr("Please wait"))

        self._base_message = self.owner.tr(text).rstrip(".")

        self.tick()

    def show_busy(self, text: str) -> None:

        self.setGeometry(self.owner.rect())

        self.set_message(text)

        self.position_panel()

        self.show()

        self.raise_()

        if not self.timer.isActive():

            self.timer.start()

    def hide_busy(self) -> None:

        self.timer.stop()

        self.hide()

    def tick(self) -> None:

        dots = "." * ((self._phase // 12) % 4)

        self.progress.set_phase(self._phase)

        if self._base_message:

            self.message_label.setText(f"{self._base_message}{dots}")

        self._phase += 1

    def position_panel(self) -> None:

        self.panel.adjustSize()

        panel_w = self.panel.width()

        panel_h = max(self.panel.sizeHint().height(), 132)

        self.panel.setGeometry(

            max(20, (self.width() - panel_w) // 2),

            max(20, (self.height() - panel_h) // 2),

            panel_w,

            panel_h,

        )


class OnboardingOverlay(QWidget):

    def __init__(self, owner: "MainWindow", steps: list[dict], finish_callback) -> None:

        super().__init__(owner)

        self.owner = owner

        self.steps = steps

        self.finish_callback = finish_callback

        self.index = 0

        self.current_target: QWidget | None = None

        self.highlight_rect = QRect()

        self.setObjectName("OnboardingOverlay")

        self.setAttribute(Qt.WA_StyledBackground, False)

        self.setFocusPolicy(Qt.StrongFocus)

        self.owner.installEventFilter(self)

        self.card = QFrame(self)

        self.card.setObjectName("TourCard")

        self.card.setFixedWidth(430)

        card_layout = QVBoxLayout(self.card)

        card_layout.setContentsMargins(18, 18, 18, 16)

        card_layout.setSpacing(12)

        header = QHBoxLayout()

        self.icon_label = QLabel()

        self.icon_label.setObjectName("TourIcon")

        self.icon_label.setAlignment(Qt.AlignCenter)

        header.addWidget(self.icon_label)

        title_box = QVBoxLayout()

        title_box.setContentsMargins(0, 0, 0, 0)

        title_box.setSpacing(2)

        self.title_label = QLabel()

        self.title_label.setObjectName("Title")

        self.progress_label = QLabel()

        self.progress_label.setObjectName("MetaLabel")

        title_box.addWidget(self.title_label)

        title_box.addWidget(self.progress_label)

        header.addLayout(title_box, 1)

        card_layout.addLayout(header)

        self.body_label = QLabel()

        self.body_label.setObjectName("TourBody")

        self.body_label.setWordWrap(True)

        card_layout.addWidget(self.body_label)

        self.route_title = QLabel("Tour route")

        self.route_title.setObjectName("TourRouteTitle")

        card_layout.addWidget(self.route_title)

        self.step_labels: list[QLabel] = []

        self.step_list = QVBoxLayout()

        self.step_list.setContentsMargins(0, 0, 0, 0)

        self.step_list.setSpacing(4)

        for _step in self.steps:

            step_label = QLabel()

            step_label.setObjectName("TourStepLabel")

            self.step_labels.append(step_label)

            self.step_list.addWidget(step_label)

        card_layout.addLayout(self.step_list)

        self.hint_label = QLabel(owner.tr("You can replay this guide later from Settings / Tools."))

        self.hint_label.setObjectName("MetaLabel")

        self.hint_label.setWordWrap(True)

        card_layout.addWidget(self.hint_label)

        actions = QHBoxLayout()

        self.skip_btn = QPushButton(owner.tr("Skip"))

        self.back_btn = QPushButton(owner.tr("Back"))

        self.next_btn = PrimaryButton(owner.tr("Next"))

        actions.addWidget(self.skip_btn)

        actions.addStretch(1)

        actions.addWidget(self.back_btn)

        actions.addWidget(self.next_btn)

        card_layout.addLayout(actions)

        self.skip_btn.clicked.connect(self.skip)

        self.back_btn.clicked.connect(self.back)

        self.next_btn.clicked.connect(self.next_step)

    def polish_widget(self, widget: QWidget | None) -> None:

        if widget is None:

            return

        widget.style().unpolish(widget)

        widget.style().polish(widget)

        widget.update()

    def set_target_highlight(self, target: QWidget | None) -> None:

        if self.current_target is target:

            return

        if self.current_target is not None:

            self.current_target.setProperty("tourHighlight", False)

            self.polish_widget(self.current_target)

        self.current_target = target

        if self.current_target is not None:

            self.current_target.setProperty("tourHighlight", True)

            self.polish_widget(self.current_target)

    def clear_page_focus(self) -> None:

        focused = QApplication.focusWidget()

        if focused is not None and not self.card.isAncestorOf(focused):

            focused.clearFocus()

        self.setFocus(Qt.OtherFocusReason)

    def eventFilter(self, watched, event) -> bool:

        if watched is self.owner and event.type() in (QEvent.Resize, QEvent.Move):

            self.setGeometry(self.owner.rect())

            self.reposition()

        return super().eventFilter(watched, event)

    def keyPressEvent(self, event) -> None:

        if event.key() == Qt.Key_Escape:

            self.skip()

            return

        if event.key() in (Qt.Key_Return, Qt.Key_Enter, Qt.Key_Space):

            self.next_step()

            return

        super().keyPressEvent(event)

    def paintEvent(self, event) -> None:

        painter = QPainter(self)

        painter.setRenderHint(QPainter.Antialiasing, True)

        painter.fillRect(self.rect(), QColor(0, 0, 0, 78))

    def start(self, index: int = 0) -> None:

        self.setGeometry(self.owner.rect())

        self.show()

        self.raise_()

        self.setFocus()

        self.show_step(index)

    def target_widget(self, step: dict) -> QWidget | None:

        target = step.get("target")

        if callable(target):

            widget = target()

            return widget if isinstance(widget, QWidget) else None

        return target if isinstance(target, QWidget) else None

    def show_step(self, index: int) -> None:

        self.index = max(0, min(index, len(self.steps) - 1))

        step = self.steps[self.index]

        page = step.get("page")

        if isinstance(page, QWidget):

            self.owner.switchTo(page)

        self.clear_page_focus()

        target = self.target_widget(step)

        self.ensure_target_visible(target)

        self.set_target_highlight(target)

        icon = step["icon"]

        self.icon_label.setPixmap(icon.qicon().pixmap(34, 34))

        self.title_label.setText(self.owner.tr(str(step["title"])))

        self.body_label.setText(self.owner.tr(str(step["body"])))

        self.progress_label.setText(f"{self.index + 1} / {len(self.steps)}")

        self.skip_btn.setText(self.owner.tr("Skip"))

        self.back_btn.setText(self.owner.tr("Back"))

        self.next_btn.setText(self.owner.tr("Finish") if self.index == len(self.steps) - 1 else self.owner.tr("Next"))

        self.back_btn.setEnabled(self.index > 0)

        self.route_title.setText(self.owner.tr("Tour route"))

        for i, label in enumerate(self.step_labels):

            marker = "✓" if i < self.index else ("●" if i == self.index else "○")

            label.setText(f"{marker} {self.owner.tr(str(self.steps[i]['title']))}")

            label.setProperty("active", i == self.index)

            self.polish_widget(label)

        self.reposition()

        self.update()

    def ensure_target_visible(self, target: QWidget | None) -> None:

        if target is None:

            return

        parent = target.parentWidget()

        while parent is not None:

            if isinstance(parent, QScrollArea):

                parent.ensureWidgetVisible(target, 32, 32)

                return

            parent = parent.parentWidget()

    def reposition(self) -> None:

        step = self.steps[self.index] if self.steps else {}

        target = self.target_widget(step)

        rect_provider = step.get("rect")

        if callable(rect_provider):

            rect = rect_provider()

            self.highlight_rect = rect if isinstance(rect, QRect) else QRect()

        elif target is not None and target.isVisible():

            top_left = target.mapTo(self, QPoint(0, 0))

            self.highlight_rect = QRect(top_left, target.size())

            if isinstance(target, QAbstractItemView):

                step.setdefault("max_width", 620)

                step.setdefault("max_height", 280)

        else:

            self.highlight_rect = QRect(self.width() // 2 - 120, self.height() // 2 - 45, 240, 90)

        max_width = step.get("max_width")

        max_height = step.get("max_height")

        align = step.get("align", "left")

        if isinstance(max_width, int) and self.highlight_rect.width() > max_width:

            if align == "right":

                self.highlight_rect.setLeft(self.highlight_rect.right() - max_width)

            else:

                self.highlight_rect.setWidth(max_width)

        if isinstance(max_height, int) and self.highlight_rect.height() > max_height:

            if align == "bottom":

                self.highlight_rect.setTop(self.highlight_rect.bottom() - max_height)

            else:

                self.highlight_rect.setHeight(max_height)

        if self.highlight_rect.width() < 120:

            delta = (120 - self.highlight_rect.width()) // 2

            self.highlight_rect.adjust(-delta, 0, delta, 0)

        if self.highlight_rect.height() < 58:

            delta = (58 - self.highlight_rect.height()) // 2

            self.highlight_rect.adjust(0, -delta, 0, delta)

        self.highlight_rect = self.highlight_rect.intersected(self.rect().adjusted(12, 12, -12, -12))

        self.card.adjustSize()

        margin = 24

        card_size = self.card.sizeHint()

        card_w = self.card.width()

        card_h = min(max(card_size.height(), self.card.height(), 420), max(420, self.height() - margin * 2))

        x = max(margin, self.width() - card_w - margin)

        y = max(margin, (self.height() - card_h) // 2)

        chosen = QRect(x, y, card_w, card_h)

        self.card.setGeometry(chosen)

    def skip(self) -> None:

        self.finish_callback()

        self.close()

    def back(self) -> None:

        self.show_step(self.index - 1)

    def next_step(self) -> None:

        if self.index >= len(self.steps) - 1:

            self.finish_callback()

            self.close()

            return

        self.show_step(self.index + 1)

    def closeEvent(self, event) -> None:

        self.set_target_highlight(None)

        self.owner.removeEventFilter(self)

        super().closeEvent(event)


class MainWindow(MSFluentWindow):

    columns = ["Fav", "Test", "Preset", "Status", "OK", "FAIL", "UNSUP", "Ping", "Services", "Score", ""]



    def __init__(self, paths: AppPaths) -> None:

        super().__init__()

        self.paths = paths

        self.current_preset = state.read_current_preset(paths.current_preset)

        self.settings = settings.load_settings(paths.gui_settings)

        setTheme(Theme.LIGHT if self.settings.theme == "light" else Theme.DARK)

        setThemeColor("#0078d4")

        self.presets: list[presets.Preset] = []

        self.results: dict[str, stored_results.ResultTuple] = stored_results.load_results(paths.gui_results)

        self.result_details: dict[str, stored_results.TargetDetails] = stored_results.load_result_details(paths.gui_results)

        self.enrich_result_details_with_targets()

        self.recalculate_results_from_details(save=True)

        self.active_filter = "all"

        self.winws_running = False

        self.tg_ws_running = False

        self.nav_items = {}

        self.worker_thread: QThread | None = None

        self.worker: TestAllWorker | None = None

        self.tests_pending_finish = False

        self.update_thread: QThread | None = None

        self.update_worker: UpdateCheckWorker | None = None

        self.update_pending_result = None

        self.service_thread: QThread | None = None

        self.service_worker: ServiceOperationWorker | None = None

        self.service_finished_callback = None

        self.service_failed_callback = None

        self.service_pending_result = None

        self.tests_running = False

        self.test_stop_requested = False

        self.close_after_tests_stop = False

        self.force_quit = False

        self._loading_table = False

        self.page_animation: QPropertyAnimation | None = None

        self.settings_page_animation: QPropertyAnimation | None = None

        self.activity_page_animation: QPropertyAnimation | None = None

        self.status_animations: dict[QWidget, QPropertyAnimation] = {}

        self._home_zapret_running = False

        self._home_tg_running = False

        self.completed_this_run = 0

        self.testing_badge_tick = 0

        self.testing_badge_timer = QTimer(self)

        self.testing_badge_timer.setInterval(450)

        self.testing_badge_timer.timeout.connect(self.update_testing_badge)

        self.setWindowTitle("Z2 GUI")

        icon_path = self.asset_path("Z2.png")

        if icon_path.exists():

            self.setWindowIcon(QIcon(str(icon_path)))



        self.admin_label = QLabel()

        self.running_label = QLabel()

        self.progress_label = QLabel("Idle")

        self.progress_bar = QProgressBar()

        self.busy_label = QLabel("")

        self.busy_label.setObjectName("MetaLabel")

        self.busy_label.setVisible(False)

        self.busy_progress = QProgressBar()

        self.busy_progress.setRange(0, 0)

        self.busy_progress.setMaximumWidth(140)

        self.busy_progress.setVisible(False)

        self.busy_overlay = BusyOverlay(self)

        self.testing_progress_label = QLabel("Idle")

        self.testing_progress_bar = QProgressBar()

        self.testing_selected_label = QLabel("Selected presets: -")

        self.testing_targets_label = QLabel("Targets: -")

        self.testing_best_label = QLabel("Best result: -")

        self.table_model = PresetTableModel(self.columns, self)

        self.table_proxy = PresetFilterProxyModel(self)

        self.table_proxy.setSourceModel(self.table_model)

        self.table = QTableView()

        self.table.setModel(self.table_proxy)

        self.filter_timer = QTimer(self)

        self.filter_timer.setSingleShot(True)

        self.filter_timer.setInterval(180)

        self.filter_timer.timeout.connect(self.apply_filter_now)

        self._pending_filter = self.active_filter

        self.log_flush_timer = QTimer(self)

        self.log_flush_timer.setInterval(120)

        self.log_flush_timer.timeout.connect(self.flush_log_messages)

        self._zapret_log_buffer: list[str] = []

        self._tg_log_buffer: list[str] = []

        self._busy_depth = 0

        self._status_pixmaps: dict[bool, QPixmap | None] = {}

        self._service_cards: dict[QLabel, GlowServiceCard] = {}

        self.log = QTextEdit()

        self.log.setReadOnly(True)

        self.activity_log = QTextEdit()

        self.activity_log.setReadOnly(True)

        self.tg_activity_log = QTextEdit()

        self.tg_activity_log.setReadOnly(True)

        for log_widget in (self.log, self.activity_log, self.tg_activity_log):

            log_widget.document().setMaximumBlockCount(1200)

            log_widget.setContextMenuPolicy(Qt.NoContextMenu)

        self.activity_zapret_btn = QtPushButton("zapret")

        self.activity_tg_btn = QtPushButton("tg-ws")

        self.activity_stack = QStackedWidget()



        self.test_all_btn = PrimaryButton("Test Selected")

        self.testing_test_btn = PrimaryButton("Test Selected")

        self.test_settings_btn = QPushButton("Profile: All targets")

        self.testing_settings_btn = QPushButton("Test Settings: All")

        self.run_selected_btn = PrimaryButton("Run Preset")

        self.testing_run_btn = PrimaryButton("Run Preset")

        self.home_run_btn = PrimaryButton("Run Preset")

        self.home_power_btn = PrimaryButton("Start")

        self.stop_btn = QPushButton("Stop")

        self.testing_stop_btn = QPushButton("Stop")

        self.home_stop_btn = QPushButton("Stop")

        self.home_test_btn = PrimaryButton("Test Selected")

        self.home_settings_btn = QPushButton("Test Settings")

        self.add_preset_btn = QPushButton("Add Preset")

        self.open_presets_btn = QPushButton("Open Presets Folder")

        self.remove_preset_btn = QPushButton("Delete/Hide")

        self.restore_hidden_btn = QPushButton("Restore Hidden")

        self.select_all_btn = QPushButton("Select All")

        self.select_favorites_btn = QPushButton("Select Fav")

        self.clear_selection_btn = QPushButton("Clear")

        self.refresh_btn = QPushButton("Refresh")

        self.relaunch_admin_btn = QPushButton("Restart as Admin")

        self.selected_title = QLabel("No preset selected")

        self.selected_status = QLabel("Status: -")

        self.selected_score = QLabel("Score: -")

        self.selected_checks = QLabel("Checks: -")

        self.selected_ping = QLabel("Ping: -")

        self.selected_path = QLabel("Path: -")

        self.selected_file_info = QLabel("File: -")

        self.home_state_label = QLabel("State: -")

        self.home_active_label = QLabel("Active preset: -")

        self.home_zapret_status_label = QLabel()

        self.home_zapret_detail_label = QLabel("Active preset: -")

        self.home_tg_status_label = QLabel()

        self.home_tg_detail_label = QLabel("Telegram proxy integration is not enabled yet")

        self.home_zapret_start_check = SwitchButton()

        self.home_tg_start_check = SwitchButton()

        self.onboarding_test_label = QLabel("1")

        self.onboarding_favorite_label = QLabel("2")

        self.onboarding_start_label = QLabel("3")

        self.readiness_admin_label = QLabel("Admin: -")

        self.readiness_preset_label = QLabel("Preset: -")

        self.readiness_targets_label = QLabel("Targets: -")

        self.readiness_services_label = QLabel("Services: -")

        self.home_presets_label = QLabel("Presets: -")

        self.home_favorites_label = QLabel("Favorites: -")

        self.home_results_label = QLabel("Tested: -")

        self.home_best_label = QLabel("Best preset: -")

        self.home_targets_label = QLabel("Targets: -")

        self.home_selected_label = QLabel("Selected preset: -")

        self.tg_status_label = QLabel("Status: -")

        self.tg_endpoint_label = QLabel("127.0.0.1:1443")

        self.tg_link_label = QLabel("Proxy link: -")

        self.tg_port_input = QSpinBox()

        self.tg_secret_input = LineEdit()

        self.settings_tg_status_label = QLabel("Status: -")

        self.settings_tg_endpoint_label = QLabel("127.0.0.1:1443")

        self.settings_tg_link_label = QLabel("Proxy link: -")

        self.settings_tg_port_input = QSpinBox()

        self.settings_tg_secret_input = LineEdit()

        self.settings_status_label = QLabel("winws2: -")

        self.settings_counts_label = QLabel("Presets: -")

        self.settings_paths_label = QLabel("Root: -")

        self.settings_about_label = QLabel(f"Z2 GUI {APP_VERSION} | Author: amantesnihilo")

        self.settings_runtime_label = QLabel(f"Python: {sys.version.split()[0]} | Platform: {platform.platform()}")

        self.settings_theme_combo = FluentComboBox()

        self.settings_language_combo = FluentComboBox()

        self.settings_close_combo = FluentComboBox()

        self.check_updates_btn = PrimaryButton("Check for Updates")

        self.home_preset_combo = FluentComboBox()

        self.empty_state_label = QLabel("No presets match the current filter or search.")

        self.empty_state_label.setObjectName("MetaLabel")

        self.empty_state_label.setAlignment(Qt.AlignCenter)

        self.empty_state_label.setVisible(False)

        self.home_favorites_grid = QGridLayout()

        self.home_favorites_grid.setContentsMargins(0, 0, 0, 0)

        self.home_favorites_grid.setHorizontalSpacing(8)

        self.home_favorites_grid.setVerticalSpacing(8)

        self.preset_preview = QTextEdit()

        self.preset_preview.setReadOnly(True)

        self.preset_preview.setMinimumHeight(64)

        self.preset_preview.setMaximumHeight(110)

        self.side_run_btn = PrimaryButton("Run")

        self.side_copy_path_btn = QPushButton("Copy Path")

        self.side_open_file_btn = QPushButton("Open File")

        self.side_reveal_file_btn = QPushButton("Reveal in Folder")

        self.side_test_details_btn = QPushButton("Test Details")

        self.side_delete_btn = QPushButton("Delete/Hide")

        self.search_input = QLineEdit()

        self.search_input.setPlaceholderText("Search presets, status, path...")

        self.search_input.setClearButtonEnabled(True)

        for button in (self.activity_zapret_btn, self.activity_tg_btn):

            button.setCheckable(True)

            button.setObjectName("ActivitySegmentButton")

            button.setCursor(Qt.PointingHandCursor)

        self.activity_zapret_btn.setChecked(True)

        self.activity_stack.setObjectName("ActivityStack")

        self.test_all_btn.setObjectName("PrimaryButton")

        self.testing_test_btn.setObjectName("PrimaryButton")

        self.run_selected_btn.setObjectName("PrimaryButton")

        self.testing_run_btn.setObjectName("PrimaryButton")

        self.home_run_btn.setObjectName("PrimaryButton")

        self.home_power_btn.setObjectName("HomePowerButton")

        self.home_test_btn.setObjectName("PrimaryButton")

        self.side_run_btn.setObjectName("PrimaryButton")

        self.stop_btn.setObjectName("DangerButton")

        self.testing_stop_btn.setObjectName("DangerButton")

        self.home_stop_btn.setObjectName("DangerButton")

        self.side_delete_btn.setObjectName("DangerButton")

        self.test_all_btn.setIcon(FluentIcon.SYNC)

        self.testing_test_btn.setIcon(FluentIcon.SYNC)

        self.run_selected_btn.setIcon(FluentIcon.PLAY)

        self.testing_run_btn.setIcon(FluentIcon.PLAY)

        self.home_run_btn.setIcon(FluentIcon.PLAY)

        self.home_power_btn.setIcon(FluentIcon.PLAY)

        self.home_test_btn.setIcon(FluentIcon.SYNC)

        self.side_run_btn.setIcon(FluentIcon.PLAY)

        self.stop_btn.setIcon(FluentIcon.CANCEL)

        self.testing_stop_btn.setIcon(FluentIcon.CANCEL)

        self.home_stop_btn.setIcon(FluentIcon.CANCEL)

        self.home_settings_btn.setIcon(FluentIcon.SETTING)

        self.check_updates_btn.setIcon(FluentIcon.SYNC)

        self.home_zapret_start_check.setChecked(self.settings.start_zapret_enabled)

        self.home_tg_start_check.setChecked(self.settings.start_tg_ws_enabled)

        for switch in (self.home_zapret_start_check, self.home_tg_start_check):

            switch.setOnText("")

            switch.setOffText("")

        self.home_zapret_start_check.setObjectName("HomeServiceToggle")

        self.home_tg_start_check.setObjectName("HomeServiceToggle")

        self.home_tg_start_check.setToolTip("tg-ws startup will work after tg-ws integration is added.")

        for label in (self.tg_status_label, self.settings_tg_status_label):

            label.setObjectName("TgStatusBadge")

            label.setAlignment(Qt.AlignCenter)

        for label in (self.tg_endpoint_label, self.settings_tg_endpoint_label):

            label.setObjectName("TgEndpointBadge")

            label.setAlignment(Qt.AlignCenter)

        for spin in (self.tg_port_input, self.settings_tg_port_input):

            spin.setRange(1, 65535)

            spin.setValue(self.settings.tg_ws_port)

            spin.setMinimumWidth(130)

        for edit in (self.tg_secret_input, self.settings_tg_secret_input):

            edit.setText(self.settings.tg_ws_secret)

            edit.setPlaceholderText("32 hex chars")

            edit.setMinimumWidth(340)

            edit.setClearButtonEnabled(True)

        self.add_preset_btn.setIcon(FluentIcon.ADD)

        self.open_presets_btn.setIcon(FluentIcon.FOLDER)

        self.refresh_btn.setIcon(FluentIcon.SYNC)

        self.side_copy_path_btn.setIcon(FluentIcon.COPY)

        self.side_open_file_btn.setIcon(FluentIcon.DOCUMENT)

        self.side_reveal_file_btn.setIcon(FluentIcon.FOLDER)

        self.side_test_details_btn.setIcon(FluentIcon.INFO)

        self.side_delete_btn.setIcon(FluentIcon.DELETE)



        self._build_ui()

        self._connect()

        self.setup_tray()

        self.reload_presets()

        QTimer.singleShot(650, self.maybe_show_onboarding)



    def _build_ui(self) -> None:

        root = QWidget()

        root.setObjectName("presetsInterface")

        layout = QVBoxLayout(root)

        layout.setContentsMargins(12, 12, 12, 12)

        layout.setSpacing(10)



        top_bar = QHBoxLayout()

        title = QLabel("Z2 GUI")

        title.setObjectName("Title")

        top_bar.addWidget(title)

        top_bar.addSpacing(18)

        top_bar.addWidget(self.running_label)

        top_bar.addStretch(1)

        top_bar.addWidget(self.admin_label)

        top_bar.addWidget(self.relaunch_admin_btn)

        layout.addLayout(top_bar)



        primary_panel = QFrame()

        primary_panel.setObjectName("CommandBar")

        primary_actions = QHBoxLayout(primary_panel)

        primary_actions.setContentsMargins(10, 8, 10, 8)

        primary_actions.addWidget(self.test_all_btn)

        primary_actions.addWidget(self.stop_btn)

        primary_actions.addSpacing(14)

        primary_actions.addWidget(self.test_settings_btn)

        primary_actions.addStretch(1)

        primary_actions.addWidget(self.refresh_btn)

        layout.addWidget(primary_panel)



        filters_panel = QFrame()

        filters_panel.setObjectName("CommandBar")

        filters_outer = QVBoxLayout(filters_panel)

        filters_outer.setContentsMargins(10, 8, 10, 8)

        filters_outer.setSpacing(7)



        filter_row = QHBoxLayout()

        show_label = QLabel("Show")

        show_label.setObjectName("InlineLabel")

        filter_row.addWidget(show_label)

        self.filter_all_btn = QPushButton("All")

        self.filter_passed_btn = QPushButton("Passed")

        self.filter_working_btn = QPushButton("Passed + Partial")

        self.filter_favorites_btn = QPushButton("Favorites")

        self.filter_buttons = QButtonGroup(self)

        self.filter_buttons.setExclusive(True)

        for button in (

            self.filter_all_btn,

            self.filter_passed_btn,

            self.filter_working_btn,

            self.filter_favorites_btn,

        ):

            button.setCheckable(True)

            button.setObjectName("SegmentButton")

            filter_row.addWidget(button)

            self.filter_buttons.addButton(button)

        self.filter_all_btn.setChecked(True)

        filter_row.addSpacing(10)

        search_label = QLabel("Search")

        search_label.setObjectName("InlineLabel")

        filter_row.addWidget(search_label)

        self.search_input.setMaximumWidth(520)

        filter_row.addWidget(self.search_input, 1)

        filters_outer.addLayout(filter_row)



        selection_row = QHBoxLayout()

        selection_label = QLabel("Selection")

        selection_label.setObjectName("InlineLabel")

        selection_row.addWidget(selection_label)

        self.select_all_btn.setText("All")

        self.select_favorites_btn.setText("Favorites")

        self.clear_selection_btn.setText("None")

        selection_row.addWidget(self.select_all_btn)

        selection_row.addWidget(self.select_favorites_btn)

        selection_row.addWidget(self.clear_selection_btn)

        selection_row.addStretch(1)

        selection_row.addWidget(self.add_preset_btn)

        selection_row.addWidget(self.open_presets_btn)

        selection_row.addWidget(self.restore_hidden_btn)

        filters_outer.addLayout(selection_row)

        layout.addWidget(filters_panel)



        progress_row = QHBoxLayout()

        progress_row.addWidget(self.progress_label)

        progress_row.addWidget(self.progress_bar, 1)

        progress_row.addWidget(self.busy_label)

        progress_row.addWidget(self.busy_progress)

        layout.addLayout(progress_row)



        self.table.setSelectionBehavior(QTableView.SelectRows)

        self.table.setSelectionMode(QTableView.SingleSelection)

        self.table.setEditTriggers(QTableView.NoEditTriggers)

        self.table.setAlternatingRowColors(False)

        self.table.setShowGrid(False)
        self.table.setFrameShape(QFrame.NoFrame)
        self.table.setMouseTracking(True)

        self.table.verticalHeader().setVisible(False)

        self.table.verticalHeader().setDefaultSectionSize(38)

        self.table.setSortingEnabled(True)

        header = self.table.horizontalHeader()

        header.setSectionResizeMode(0, QHeaderView.ResizeToContents)

        header.setSectionResizeMode(1, QHeaderView.ResizeToContents)

        header.setSectionResizeMode(2, QHeaderView.Stretch)

        self.table.setColumnWidth(2, 320)

        for column in range(3, 10):

            header.setSectionResizeMode(column, QHeaderView.Interactive)

        self.table.setColumnWidth(3, 90)

        self.table.setColumnWidth(4, 56)

        self.table.setColumnWidth(5, 60)

        self.table.setColumnWidth(6, 70)

        self.table.setColumnWidth(7, 68)

        self.table.setColumnWidth(8, 150)

        self.table.setColumnWidth(9, 72)

        header.setSectionResizeMode(10, QHeaderView.Fixed)

        self.table.setColumnWidth(10, 42)

        header.moveSection(header.visualIndex(10), 3)



        left_content = QWidget()

        left_layout = QVBoxLayout(left_content)

        left_layout.setContentsMargins(0, 0, 0, 0)

        left_layout.setSpacing(10)

        left_layout.addWidget(self.table, 1)

        left_layout.addWidget(self.empty_state_label)

        layout.addWidget(left_content, 1)



        self.home_interface = self._build_home_interface()

        self.presets_interface = root

        self.tg_interface = self._build_tg_interface()

        self.testing_interface = self._build_testing_interface()

        self.activity_interface = self._build_activity_interface()

        self.settings_interface = self._build_settings_interface()

        self.configure_smooth_scrolling()



        self.nav_items["home"] = self.addSubInterface(

            self.home_interface,

            FluentIcon.HOME,

            "Home",

            position=NavigationItemPosition.TOP,

        )

        self.nav_items["presets"] = self.addSubInterface(self.presets_interface, FluentIcon.APPLICATION, "Presets", position=NavigationItemPosition.TOP)

        self.nav_items["tg"] = self.addSubInterface(self.tg_interface, FluentIcon.SEND, "TG", position=NavigationItemPosition.TOP)

        self.nav_items["activity"] = self.addSubInterface(

            self.activity_interface,

            FluentIcon.HISTORY,

            "Activity",

            position=NavigationItemPosition.TOP,

        )

        self.nav_items["settings"] = self.addSubInterface(

            self.settings_interface,

            FluentIcon.SETTING,

            "Settings",

            position=NavigationItemPosition.BOTTOM,

        )

        self.stackedWidget.setAnimationEnabled(False)

        self.stackedWidget.currentChanged.connect(self.animate_current_page)

        self.progress_bar.setRange(0, 1)

        self.progress_bar.setValue(0)

        self.testing_progress_bar.setRange(0, 1)

        self.testing_progress_bar.setValue(0)

        self._apply_style()

        self.apply_language()

        self.setMinimumSize(980, 560)

        self._update_admin_label()

        self._update_running_label()



    def tr(self, text: str) -> str:

        if self.settings.language == "ru":

            return RU_TEXT.get(text, text)

        return text



    def source_text(self, text: str) -> str:

        reverse = {value: key for key, value in RU_TEXT.items()}

        return reverse.get(text, text)



    def asset_path(self, name: str) -> Path:

        return self.paths.root / "gui" / "ui" / "assets" / name



    def configure_smooth_scrolling(self) -> None:

        for view in self.findChildren(QAbstractItemView):

            view.setVerticalScrollMode(QAbstractItemView.ScrollPerPixel)

            view.setHorizontalScrollMode(QAbstractItemView.ScrollPerPixel)

            view.verticalScrollBar().setSingleStep(18)

            view.horizontalScrollBar().setSingleStep(18)

        for area in self.findChildren(QScrollArea):

            area.verticalScrollBar().setSingleStep(18)

            area.horizontalScrollBar().setSingleStep(18)



    def tray_icon_for_windows(self) -> QIcon:

        image = QImage(str(self.asset_path("Z2.png")))

        if image.isNull():

            return self.windowIcon()



        min_x, min_y = image.width(), image.height()

        max_x, max_y = -1, -1

        for y in range(image.height()):

            for x in range(image.width()):

                if image.pixelColor(x, y).alpha() > 8:

                    min_x = min(min_x, x)

                    min_y = min(min_y, y)

                    max_x = max(max_x, x)

                    max_y = max(max_y, y)



        if max_x < min_x or max_y < min_y:

            return self.windowIcon()



        content_w = max_x - min_x + 1

        content_h = max_y - min_y + 1

        side = int(max(content_w, content_h) * 1.06)

        center_x = (min_x + max_x) // 2

        center_y = (min_y + max_y) // 2

        left = max(0, min(image.width() - side, center_x - side // 2))

        top = max(0, min(image.height() - side, center_y - side // 2))

        cropped = image.copy(left, top, side, side)



        icon = QIcon()

        for size in (16, 20, 24, 32, 40, 48, 64, 128, 256):

            pixmap = QPixmap.fromImage(cropped).scaled(

                size,

                size,

                Qt.KeepAspectRatio,

                Qt.SmoothTransformation,

            )

            icon.addPixmap(pixmap)

        return icon



    def apply_language(self) -> None:

        def translate_text(text: str) -> str:

            source = self.source_text(text)

            return self.tr(source)



        for widget in self.findChildren(QWidget):

            if isinstance(widget, QTableView):

                continue

            if hasattr(widget, "text") and hasattr(widget, "setText"):

                try:

                    text_attr = getattr(widget, "text")

                    text = text_attr() if callable(text_attr) else str(text_attr)

                except RuntimeError:

                    continue

                if text:

                    widget.setText(translate_text(text))

            if hasattr(widget, "placeholderText") and hasattr(widget, "setPlaceholderText"):

                try:

                    placeholder = widget.placeholderText()

                except RuntimeError:

                    continue

                if placeholder:

                    widget.setPlaceholderText(translate_text(placeholder))



        self.table_model.set_headers([self.tr(column) for column in self.columns])

        self.update_nav_texts()

        self.update_test_settings_button()

        self._update_admin_label()

        self._update_running_label()

        self.update_tg_summary()

        self.update_tray_texts()



    def update_nav_texts(self) -> None:

        labels = {

            "home": "Home",

            "presets": "Presets",

            "tg": "TG",

            "activity": "Activity",

            "settings": "Settings",

        }

        for key, source in labels.items():

            item = self.nav_items.get(key)

            if item is not None and hasattr(item, "setText"):

                item.setText(self.tr(source))



    def _build_home_interface(self) -> QWidget:

        page = QWidget()

        page.setObjectName("homeInterface")

        layout = QVBoxLayout(page)

        layout.setContentsMargins(24, 24, 24, 24)

        layout.setSpacing(14)



        header = QHBoxLayout()

        title = QLabel("Home")

        title.setObjectName("Title")

        header.addWidget(title)

        header.addStretch(1)

        layout.addLayout(header)



        services_row = QHBoxLayout()

        services_row.setSpacing(14)

        self.home_zapret_card = self._build_service_card(

                "zapret",

                self.home_zapret_status_label,

                self.home_zapret_detail_label,

                self.home_zapret_start_check,

                QColor("#28c76f"),

        )

        services_row.addWidget(self.home_zapret_card)

        self.home_tg_card = self._build_service_card(

                "tg-ws",

                self.home_tg_status_label,

                self.home_tg_detail_label,

                self.home_tg_start_check,

                QColor("#2aa8ff"),

        )

        services_row.addWidget(self.home_tg_card)

        layout.addLayout(services_row)



        launch_card = panel_card()

        launch_card.setObjectName("HomeLaunchCard")

        self.home_preset_card = launch_card

        self.home_readiness_card = launch_card

        launch_layout = launch_card.layout()

        launch_layout.setSpacing(16)

        launch_header = QHBoxLayout()

        launch_header.setContentsMargins(0, 0, 0, 0)

        launch_title = QLabel("Launch")

        launch_title.setObjectName("HomeSectionTitle")

        launch_header.addWidget(launch_title)

        launch_header.addStretch(1)

        launch_layout.addLayout(launch_header)

        launch_body = QHBoxLayout()

        launch_body.setSpacing(18)

        preset_box = QVBoxLayout()

        preset_box.setContentsMargins(0, 0, 0, 0)

        preset_box.setSpacing(10)

        preset_label = QLabel("Selected preset")

        preset_label.setObjectName("LaunchFieldLabel")

        self.home_preset_combo.setMinimumWidth(420)

        preset_box.addWidget(preset_label)

        preset_box.addWidget(self.home_preset_combo)

        self.home_selected_label.setObjectName("MetaLabel")

        self.home_selected_label.setWordWrap(True)

        preset_box.addWidget(self.home_selected_label)

        launch_body.addLayout(preset_box, 1)

        action_box = QVBoxLayout()

        action_box.setContentsMargins(0, 0, 0, 0)

        action_box.setSpacing(10)

        self.home_power_btn.setMinimumSize(260, 86)

        action_box.addWidget(self.home_power_btn)

        quick_actions = QHBoxLayout()

        quick_actions.setSpacing(8)

        quick_actions.addWidget(self.home_settings_btn)

        quick_actions.addWidget(self.home_test_btn)

        action_box.addLayout(quick_actions)

        action_box.addStretch(1)

        launch_body.addLayout(action_box)

        launch_layout.addLayout(launch_body)

        self.home_readiness_admin_item = launch_card

        layout.addWidget(launch_card)



        layout.addStretch(1)

        return page



    def _build_onboarding_step(self, number_label: QLabel, title: str, description: str, _target: QWidget) -> QWidget:

        card = QWidget()

        card.setObjectName("OnboardingStep")

        layout = QHBoxLayout(card)

        layout.setContentsMargins(12, 10, 12, 10)

        layout.setSpacing(10)



        number_label.setObjectName("OnboardingStepNumber")

        number_label.setProperty("stepNumber", number_label.text())

        number_label.setAlignment(Qt.AlignCenter)

        layout.addWidget(number_label)



        text_box = QVBoxLayout()

        text_box.setContentsMargins(0, 0, 0, 0)

        text_box.setSpacing(2)

        title_label = QLabel(title)

        title_label.setObjectName("OnboardingStepTitle")

        desc_label = QLabel(description)

        desc_label.setObjectName("MetaLabel")

        desc_label.setWordWrap(True)

        text_box.addWidget(title_label)

        text_box.addWidget(desc_label)

        layout.addLayout(text_box, 1)

        return card


    def _build_readiness_item(self, icon: FluentIcon, title: str, value_label: QLabel) -> QWidget:

        item = QWidget()

        item.setObjectName("ReadinessItem")

        layout = QHBoxLayout(item)

        layout.setContentsMargins(12, 10, 12, 10)

        layout.setSpacing(10)

        icon_label = QLabel()

        icon_label.setObjectName("ReadinessIcon")

        icon_label.setAlignment(Qt.AlignCenter)

        icon_label.setPixmap(icon.qicon().pixmap(20, 20))

        layout.addWidget(icon_label)

        text_box = QVBoxLayout()

        text_box.setContentsMargins(0, 0, 0, 0)

        text_box.setSpacing(2)

        title_label = QLabel(title)

        title_label.setObjectName("OnboardingStepTitle")

        value_label.setObjectName("MetaLabel")

        value_label.setWordWrap(True)

        text_box.addWidget(title_label)

        text_box.addWidget(value_label)

        layout.addLayout(text_box, 1)

        return item



    def _build_service_card(

        self,

        title: str,

        status_label: QLabel,

        detail_label: QLabel,

        start_check: QCheckBox,

        accent: QColor,

    ) -> QWidget:

        card = GlowServiceCard(accent)

        card.setObjectName("HomeServiceCard")

        layout = card.layout()

        layout.setSpacing(10)

        self._service_cards[status_label] = card

        top_row = QHBoxLayout()

        top_row.setContentsMargins(0, 0, 0, 0)

        top_row.setSpacing(10)

        title_label = QLabel(title)

        title_label.setObjectName("HomeServiceTitle")

        title_label.setAlignment(Qt.AlignLeft | Qt.AlignVCenter)

        status_label.setObjectName("HomeServiceStatusOff")

        status_label.setAlignment(Qt.AlignCenter)

        status_label.setWordWrap(True)

        status_label.setMinimumSize(92, 30)

        status_label.setMaximumHeight(34)

        detail_label.setObjectName("MetaLabel")

        detail_label.setAlignment(Qt.AlignLeft | Qt.AlignVCenter)

        detail_label.setWordWrap(True)

        start_check.setCursor(Qt.PointingHandCursor)

        top_row.addWidget(title_label)

        layout.addLayout(top_row)

        layout.addWidget(detail_label)

        status_label.setVisible(False)

        toggle_row = QHBoxLayout()

        toggle_row.setContentsMargins(0, 0, 0, 0)

        toggle_row.setSpacing(10)

        toggle_row.addWidget(start_check, 0, Qt.AlignLeft)

        toggle_label = QLabel("Start with main button")

        toggle_label.setObjectName("MetaLabel")

        toggle_row.addWidget(toggle_label)

        toggle_row.addStretch(1)

        layout.addLayout(toggle_row)

        layout.addStretch(1)

        return card



    def _build_testing_interface(self) -> QWidget:

        page = QWidget()

        page.setObjectName("testingInterface")

        layout = QVBoxLayout(page)

        layout.setContentsMargins(24, 24, 24, 24)

        layout.setSpacing(14)



        title = QLabel("Testing")

        title.setObjectName("Title")

        layout.addWidget(title)



        status_card = panel_card("Current Run")

        status_layout = status_card.layout()

        status_layout.addWidget(self.testing_progress_label)

        status_layout.addWidget(self.testing_progress_bar)

        for label in (self.testing_selected_label, self.testing_targets_label, self.testing_best_label):

            label.setWordWrap(True)

            label.setObjectName("MetaLabel")

            status_layout.addWidget(label)

        layout.addWidget(status_card)



        actions_card = panel_card("Actions")

        actions_layout = actions_card.layout()

        actions_layout.setSpacing(12)



        action_row = QHBoxLayout()

        action_row.addWidget(self.testing_test_btn)

        action_row.addWidget(self.testing_run_btn)

        action_row.addWidget(self.testing_stop_btn)

        action_row.addWidget(self.testing_settings_btn)

        action_row.addStretch(1)

        actions_layout.addLayout(action_row)

        layout.addWidget(actions_card)



        tips_card = panel_card("Before Testing")

        tips_layout = tips_card.layout()

        tips_layout.setSpacing(8)

        tips_text = QLabel(

            "Z2 GUI checks for known VPN/proxy conflicts before tests and preset launch. "

            "Use Test Settings to keep runs short by selecting only the targets you need."

        )

        tips_text.setWordWrap(True)

        tips_layout.addWidget(tips_text)

        layout.addWidget(tips_card)



        layout.addStretch(1)

        return page



    def _build_activity_interface(self) -> QWidget:

        page = QWidget()

        page.setObjectName("activityInterface")

        layout = QVBoxLayout(page)

        layout.setContentsMargins(24, 24, 24, 24)

        layout.setSpacing(14)



        header = QHBoxLayout()

        title = QLabel("Activity")

        title.setObjectName("Title")

        header.addWidget(title)

        header.addStretch(1)

        clear_btn = QPushButton("Clear Log")

        clear_btn.setIcon(FluentIcon.BROOM)

        clear_btn.clicked.connect(self.clear_activity_log)

        header.addWidget(clear_btn)

        layout.addLayout(header)



        segment_bar = QFrame()

        segment_bar.setObjectName("ActivitySegmentBar")

        segment_layout = QHBoxLayout(segment_bar)

        segment_layout.setContentsMargins(6, 6, 6, 6)

        segment_layout.setSpacing(6)

        segment_layout.addWidget(self.activity_zapret_btn)

        segment_layout.addWidget(self.activity_tg_btn)

        segment_layout.addStretch(1)

        layout.addWidget(segment_bar)



        self.activity_log.setMinimumHeight(260)

        self.tg_activity_log.setMinimumHeight(260)

        self.activity_stack.addWidget(self.activity_log)

        self.activity_stack.addWidget(self.tg_activity_log)

        layout.addWidget(self.activity_stack, 1)

        return page



    def clear_activity_log(self) -> None:

        self.flush_log_messages()

        if self.activity_stack.currentIndex() == 0:

            self.log.clear()

            self.activity_log.clear()

            return

        self.tg_activity_log.clear()



    def switch_activity_console(self, index: int) -> None:

        if self.activity_stack.currentIndex() == index:

            self.activity_zapret_btn.setChecked(index == 0)

            self.activity_tg_btn.setChecked(index == 1)

            return

        self.activity_zapret_btn.setChecked(index == 0)

        self.activity_tg_btn.setChecked(index == 1)

        self.activity_stack.setCurrentIndex(index)

        page = self.activity_stack.currentWidget()

        if page is None:

            return

        effect = QGraphicsOpacityEffect(page)

        page.setGraphicsEffect(effect)

        animation = QPropertyAnimation(effect, b"opacity", self)

        animation.setDuration(170)

        animation.setStartValue(0.25)

        animation.setEndValue(1.0)

        animation.setEasingCurve(QEasingCurve.OutCubic)

        animation.finished.connect(lambda widget=page: widget.setGraphicsEffect(None))

        self.activity_page_animation = animation

        animation.start()



    def _build_tg_interface(self) -> QWidget:

        page = QWidget()

        page.setObjectName("tgInterface")

        layout = QVBoxLayout(page)

        layout.setContentsMargins(24, 24, 24, 24)

        layout.setSpacing(14)



        header = QHBoxLayout()

        title = QLabel("TG WS Proxy")

        title.setObjectName("Title")

        header.addWidget(title)

        header.addStretch(1)

        header.addWidget(self.tg_status_label)

        header.addWidget(self.tg_endpoint_label)

        layout.addLayout(header)



        layout.addWidget(

            self._build_tg_config_card(

                "Proxy Settings",

                self.tg_port_input,

                self.tg_secret_input,

                self.tg_link_label,

                include_status=False,

            )

        )

        layout.addWidget(self._build_tg_actions_card())

        layout.addStretch(1)

        return page



    def _build_tg_config_card(

        self,

        title: str,

        port_input: QSpinBox,

        secret_input: LineEdit,

        link_label: QLabel,

        include_status: bool = True,

    ) -> QWidget:

        card = panel_card(title)

        card_layout = card.layout()

        card_layout.setSpacing(12)

        if include_status:

            status_row = QHBoxLayout()

            status = self.settings_tg_status_label if link_label is self.settings_tg_link_label else self.tg_status_label

            endpoint = self.settings_tg_endpoint_label if link_label is self.settings_tg_link_label else self.tg_endpoint_label

            status_row.addWidget(status)

            status_row.addWidget(endpoint)

            status_row.addStretch(1)

            card_layout.addLayout(status_row)

        card_layout.addWidget(setting_row("Port", port_input))

        card_layout.addWidget(setting_row("Secret", secret_input))

        link_label.setObjectName("MetaLabel")

        link_label.setWordWrap(True)

        card_layout.addWidget(link_label)

        return card



    def _build_tg_actions_card(self) -> QWidget:

        card = panel_card("Actions")

        row = QHBoxLayout()

        copy_btn = QPushButton("Copy proxy link")

        copy_btn.setIcon(FluentIcon.COPY)

        copy_btn.clicked.connect(self.copy_tg_proxy_link)

        row.addWidget(copy_btn)



        open_tg_btn = QPushButton("Open in Telegram")

        open_tg_btn.setIcon(FluentIcon.SEND)

        open_tg_btn.clicked.connect(self.open_tg_proxy_link)

        row.addWidget(open_tg_btn)



        restart_btn = QPushButton("Restart tg-ws")

        restart_btn.setIcon(FluentIcon.SYNC)

        restart_btn.clicked.connect(self.restart_tg_ws_proxy)

        row.addWidget(restart_btn)



        log_btn = QPushButton("Open log")

        log_btn.setIcon(FluentIcon.DOCUMENT)

        log_btn.clicked.connect(self.open_tg_ws_log)

        row.addWidget(log_btn)

        row.addStretch(1)

        card.layout().addLayout(row)

        return card



    def _build_settings_interface(self) -> QWidget:

        page = QWidget()

        page.setObjectName("settingsInterface")

        layout = QVBoxLayout(page)

        layout.setContentsMargins(24, 24, 24, 24)

        layout.setSpacing(14)



        title = QLabel("Settings")

        title.setObjectName("Title")

        layout.addWidget(title)



        section_bar = QFrame()

        section_bar.setObjectName("SettingsNavBar")

        section_layout = QHBoxLayout(section_bar)

        section_layout.setContentsMargins(8, 8, 8, 8)

        section_layout.setSpacing(8)

        layout.addWidget(section_bar)



        settings_stack = QStackedWidget()

        settings_stack.setObjectName("SettingsStack")

        layout.addWidget(settings_stack, 1)

        section_buttons = QButtonGroup(self)

        section_buttons.setExclusive(True)



        def switch_settings_section(page_index: int) -> None:

            if settings_stack.currentIndex() == page_index:

                return

            settings_stack.setCurrentIndex(page_index)

            page_widget = settings_stack.currentWidget()

            if page_widget is None:

                return



            effect = QGraphicsOpacityEffect(page_widget)

            page_widget.setGraphicsEffect(effect)

            animation = QPropertyAnimation(effect, b"opacity", self)

            animation.setDuration(160)

            animation.setStartValue(0.35)

            animation.setEndValue(1.0)

            animation.setEasingCurve(QEasingCurve.OutCubic)

            animation.finished.connect(lambda widget=page_widget: widget.setGraphicsEffect(None))

            self.settings_page_animation = animation

            animation.start()



        def add_section(label: str, content: QWidget) -> None:

            index = settings_stack.addWidget(content)

            button = QtPushButton(label)

            button.setCheckable(True)

            button.setObjectName("SettingsNavButton")

            button.setCursor(Qt.PointingHandCursor)

            button.clicked.connect(lambda _checked=False, page_index=index: switch_settings_section(page_index))

            section_buttons.addButton(button)

            section_layout.addWidget(button)

            if index == 0:

                button.setChecked(True)



        add_section("About", self._build_settings_about_section())

        add_section("Appearance", self._build_settings_appearance_section())

        add_section("Behavior", self._build_settings_behavior_section())

        add_section("TG", self._build_settings_tg_section())

        add_section("Status", self._build_settings_status_section())

        add_section("Tools", self._build_settings_tools_section())

        section_layout.addStretch(1)



        return page



    def _settings_section_page(self) -> tuple[QWidget, QVBoxLayout]:

        page = QWidget()

        layout = QVBoxLayout(page)

        layout.setContentsMargins(0, 0, 0, 0)

        layout.setSpacing(14)

        return page, layout



    def _build_settings_about_section(self) -> QWidget:

        page, layout = self._settings_section_page()

        about_card = panel_card()

        about_layout = about_card.layout()



        header = QWidget()

        header.setObjectName("AboutHeader")

        header_layout = QHBoxLayout(header)

        header_layout.setContentsMargins(0, 0, 0, 0)

        header_layout.setSpacing(14)



        icon_label = QLabel()

        icon_label.setObjectName("AboutIcon")

        icon = self.windowIcon()

        if not icon.isNull():

            icon_label.setPixmap(icon.pixmap(48, 48))

        header_layout.addWidget(icon_label)



        title_box = QVBoxLayout()

        title_box.setContentsMargins(0, 0, 0, 0)

        title_box.setSpacing(4)

        app_title = QLabel("Z2 GUI")

        app_title.setObjectName("AboutTitle")

        app_description = QLabel(

            "Z2 GUI is a portable Windows utility for testing zapret2 presets, "

            "launching the selected working configuration, and managing tg-ws proxy from one clean interface."

        )

        app_description.setObjectName("AboutSubtitle")

        app_description.setWordWrap(True)

        title_box.addWidget(app_title)

        title_box.addWidget(app_description)

        header_layout.addLayout(title_box, 1)



        version_pill = QLabel(f"v{APP_VERSION}")

        version_pill.setObjectName("AboutVersion")

        version_pill.setAlignment(Qt.AlignCenter)

        header_layout.addWidget(version_pill)

        about_layout.addWidget(header)



        info_grid = QGridLayout()

        info_grid.setContentsMargins(0, 10, 0, 0)

        info_grid.setHorizontalSpacing(18)

        info_grid.setVerticalSpacing(10)



        def add_info(row: int, key: str, value: str) -> None:

            key_label = QLabel(key)

            key_label.setObjectName("AboutInfoKey")

            value_label = QLabel(value)

            value_label.setObjectName("AboutInfoValue")

            value_label.setWordWrap(True)

            info_grid.addWidget(key_label, row, 0)

            info_grid.addWidget(value_label, row, 1)



        add_info(0, "Author", "amantesnihilo")

        add_info(1, "Runtime", f"Python {sys.version.split()[0]}")

        add_info(2, "Platform", platform.platform())

        add_info(3, "Project root", str(self.paths.root))

        info_grid.setColumnStretch(1, 1)

        about_layout.addLayout(info_grid)



        layout.addWidget(about_card)



        credits_card = panel_card("Credits")

        credits_layout = credits_card.layout()

        credits_layout.setSpacing(10)



        credits = [

            (

                "zapret2",

                "Upstream DPI bypass toolkit by bol-van",

                "https://github.com/bol-van/zapret2",

            ),

            (

                "zapret2-youtube-discord",

                "Preset source and zapret2 bundle by youtubediscord",

                "https://github.com/youtubediscord/zapret2-youtube-discord",

            ),

            (

                "tg-ws-proxy",

                "Telegram WebSocket proxy by Flowseal, MIT License",

                "https://github.com/Flowseal/tg-ws-proxy",

            ),

        ]



        for name, description, url in credits:

            row = QWidget()

            row.setObjectName("CreditRow")

            row_layout = QHBoxLayout(row)

            row_layout.setContentsMargins(0, 0, 0, 0)

            row_layout.setSpacing(12)



            name_label = QLabel(name)

            name_label.setObjectName("CreditName")

            detail_label = QLabel(f'{description}<br><a href="{url}">{url}</a>')

            detail_label.setObjectName("CreditDetail")

            detail_label.setTextFormat(Qt.RichText)

            detail_label.setTextInteractionFlags(Qt.TextBrowserInteraction)

            detail_label.setOpenExternalLinks(True)

            detail_label.setWordWrap(True)



            row_layout.addWidget(name_label)

            row_layout.addWidget(detail_label, 1)

            credits_layout.addWidget(row)



        layout.addWidget(credits_card)

        layout.addStretch(1)

        return page



    def _build_settings_appearance_section(self) -> QWidget:

        page, layout = self._settings_section_page()

        appearance_card = panel_card("Appearance")

        appearance_layout = appearance_card.layout()

        appearance_layout.setSpacing(12)

        for label, value in THEME_OPTIONS:

            self.settings_theme_combo.addItem(label, value)

        theme_index = self.option_index(THEME_OPTIONS, self.settings.theme)

        if theme_index >= 0:

            self.settings_theme_combo.setCurrentIndex(theme_index)

        appearance_layout.addWidget(setting_row("Theme", self.settings_theme_combo))

        for label, value in LANGUAGE_OPTIONS:

            self.settings_language_combo.addItem(label, value)

        language_index = self.option_index(LANGUAGE_OPTIONS, self.settings.language)

        if language_index >= 0:

            self.settings_language_combo.setCurrentIndex(language_index)

        appearance_layout.addWidget(setting_row("Language", self.settings_language_combo))

        layout.addWidget(appearance_card)

        layout.addStretch(1)

        return page



    def _build_settings_behavior_section(self) -> QWidget:

        page, layout = self._settings_section_page()

        behavior_card = panel_card("Behavior")

        behavior_layout = behavior_card.layout()

        behavior_layout.setSpacing(12)

        for label, value in CLOSE_OPTIONS:

            self.settings_close_combo.addItem(label, value)

        close_index = self.option_index(CLOSE_OPTIONS, self.settings.close_winws_action)

        if close_index >= 0:

            self.settings_close_combo.setCurrentIndex(close_index)

        behavior_layout.addWidget(setting_row("On close with winws2", self.settings_close_combo))

        layout.addWidget(behavior_card)

        layout.addStretch(1)

        return page



    def _build_settings_tg_section(self) -> QWidget:

        page, layout = self._settings_section_page()

        layout.addWidget(

            self._build_tg_config_card(

                "TG WS Proxy",

                self.settings_tg_port_input,

                self.settings_tg_secret_input,

                self.settings_tg_link_label,

                include_status=True,

            )

        )

        layout.addWidget(self._build_tg_actions_card())

        layout.addStretch(1)

        return page



    def _build_settings_status_section(self) -> QWidget:

        page, layout = self._settings_section_page()

        status_card = panel_card("Status")

        status_layout = status_card.layout()

        for label in (self.settings_status_label, self.settings_counts_label, self.settings_paths_label):

            label.setWordWrap(True)

            label.setObjectName("MetaLabel")

            status_layout.addWidget(label)

        layout.addWidget(status_card)

        layout.addStretch(1)

        return page



    def _build_settings_tools_section(self) -> QWidget:

        page, layout = self._settings_section_page()

        tools_card = panel_card("Tools")

        tools_layout = tools_card.layout()

        tools_row = QHBoxLayout()

        open_presets_btn = QPushButton("Open Presets Folder")

        open_presets_btn.setIcon(FluentIcon.FOLDER)

        open_presets_btn.clicked.connect(self.open_presets_folder)

        tools_row.addWidget(open_presets_btn)

        restart_admin_btn = QPushButton("Restart as Admin")

        restart_admin_btn.setIcon(FluentIcon.UP)

        restart_admin_btn.clicked.connect(self.relaunch_as_admin)

        tools_row.addWidget(restart_admin_btn)

        open_settings_json_btn = QPushButton("Open Settings JSON")

        open_settings_json_btn.setIcon(FluentIcon.DOCUMENT)

        open_settings_json_btn.clicked.connect(lambda: self.open_local_file(self.paths.gui_settings, "Settings JSON"))

        tools_row.addWidget(open_settings_json_btn)

        open_results_json_btn = QPushButton("Open Results JSON")

        open_results_json_btn.setIcon(FluentIcon.DOCUMENT)

        open_results_json_btn.clicked.connect(lambda: self.open_local_file(self.paths.gui_results, "Results JSON"))

        tools_row.addWidget(open_results_json_btn)

        show_onboarding_btn = QPushButton("Show Welcome Guide")

        show_onboarding_btn.setIcon(FluentIcon.INFO)

        show_onboarding_btn.clicked.connect(lambda: self.show_onboarding_dialog(force=True))

        tools_row.addWidget(show_onboarding_btn)

        tools_row.addStretch(1)

        tools_layout.addLayout(tools_row)



        backup_row = QHBoxLayout()

        export_settings_btn = PrimaryButton("Export Settings")

        export_settings_btn.setIcon(FluentIcon.SAVE)

        export_settings_btn.clicked.connect(self.export_settings_backup)

        backup_row.addWidget(export_settings_btn)

        import_settings_btn = QPushButton("Import Settings")

        import_settings_btn.setIcon(FluentIcon.DOWNLOAD)

        import_settings_btn.clicked.connect(self.import_settings_backup)

        backup_row.addWidget(import_settings_btn)

        backup_note = QLabel("Exports settings, last test results, and custom preset files.")

        backup_note.setObjectName("MetaLabel")

        backup_note.setWordWrap(True)

        backup_row.addWidget(backup_note, 1)

        tools_layout.addLayout(backup_row)



        updates_card = panel_card("Updates")

        updates_layout = updates_card.layout()

        updates_row = QHBoxLayout()

        updates_row.addWidget(self.check_updates_btn)

        open_releases_btn = QPushButton("Open Releases")

        open_releases_btn.setIcon(FluentIcon.LINK)

        open_releases_btn.clicked.connect(lambda: QDesktopServices.openUrl(QUrl(PROJECT_RELEASES_URL)))

        updates_row.addWidget(open_releases_btn)

        updates_row.addStretch(1)

        updates_layout.addLayout(updates_row)

        updates_note = QLabel(

            "Z2 GUI checks GitHub Releases and shows CHANGES.md. "

            "Export your settings before installing a new portable build."

        )

        updates_note.setObjectName("MetaLabel")

        updates_note.setWordWrap(True)

        updates_layout.addWidget(updates_note)

        layout.addWidget(updates_card)



        layout.addWidget(tools_card)

        layout.addStretch(1)

        return page



    def option_index(self, options: list[tuple[str, str]], value: str) -> int:

        for index, (_label, option_value) in enumerate(options):

            if option_value == value:

                return index

        return -1



    def option_value(self, options: list[tuple[str, str]], index: int) -> str:

        if 0 <= index < len(options):

            return options[index][1]

        return ""



    def _build_sidebar(self) -> QWidget:

        sidebar = QFrame()

        sidebar.setObjectName("Sidebar")



        layout = QVBoxLayout(sidebar)

        layout.setContentsMargins(12, 12, 12, 12)

        layout.setSpacing(8)



        selected_card = panel_card("Selected Preset")

        self.selected_title.setObjectName("PresetTitle")

        self.selected_title.setWordWrap(True)

        self.selected_status.setWordWrap(True)

        self.selected_status.setObjectName("MetaLabel")

        selected_card.layout().addWidget(self.selected_title)

        selected_card.layout().addWidget(self.selected_status)



        result_card = panel_card("Result")

        for label in (self.selected_score, self.selected_checks, self.selected_ping):

            label.setWordWrap(True)

            label.setObjectName("MetaLabel")

            result_card.layout().addWidget(label)



        file_card = panel_card("File")

        for label in (self.selected_path, self.selected_file_info):

            label.setWordWrap(True)

            label.setObjectName("MetaLabel")

            file_card.layout().addWidget(label)

        file_card.layout().addWidget(self.preset_preview)



        actions_card = panel_card("Actions")

        actions_card.layout().addWidget(self.side_run_btn)

        actions_card.layout().addWidget(self.side_copy_path_btn)

        actions_card.layout().addWidget(self.side_open_file_btn)

        actions_card.layout().addWidget(self.side_reveal_file_btn)

        actions_card.layout().addWidget(self.side_test_details_btn)

        actions_card.layout().addWidget(self.side_delete_btn)



        layout.addWidget(selected_card)

        layout.addWidget(result_card)

        layout.addWidget(file_card)

        layout.addWidget(actions_card)

        layout.addStretch(1)



        scroll = QScrollArea()

        scroll.setObjectName("SidebarScroll")

        scroll.setWidgetResizable(True)

        scroll.setFrameShape(QFrame.NoFrame)

        scroll.setHorizontalScrollBarPolicy(Qt.ScrollBarAlwaysOff)

        scroll.setWidget(sidebar)

        scroll.setMinimumWidth(200)

        scroll.setMaximumWidth(270)

        return scroll



    def _connect(self) -> None:

        self.refresh_btn.clicked.connect(self.reload_presets)

        self.add_preset_btn.clicked.connect(self.add_preset)

        self.open_presets_btn.clicked.connect(self.open_presets_folder)

        self.remove_preset_btn.clicked.connect(self.delete_or_hide_selected)

        self.restore_hidden_btn.clicked.connect(self.restore_hidden_presets)

        self.select_all_btn.clicked.connect(self.select_all_presets)

        self.select_favorites_btn.clicked.connect(self.select_favorites)

        self.clear_selection_btn.clicked.connect(self.clear_preset_selection)

        self.run_selected_btn.clicked.connect(self.run_selected)

        self.testing_run_btn.clicked.connect(self.run_selected)

        self.home_run_btn.clicked.connect(self.run_selected)

        self.home_power_btn.clicked.connect(self.toggle_home_power)

        self.stop_btn.clicked.connect(self.stop)

        self.testing_stop_btn.clicked.connect(self.stop)

        self.home_stop_btn.clicked.connect(self.stop)

        self.test_all_btn.clicked.connect(self.test_all_placeholder)

        self.testing_test_btn.clicked.connect(self.test_all_placeholder)

        self.test_settings_btn.clicked.connect(self.open_test_settings)

        self.testing_settings_btn.clicked.connect(self.open_test_settings)

        self.home_settings_btn.clicked.connect(self.home_secondary_action)

        self.settings_theme_combo.currentIndexChanged.connect(self.on_settings_theme_changed)

        self.settings_language_combo.currentIndexChanged.connect(self.on_settings_language_changed)

        self.settings_close_combo.currentIndexChanged.connect(self.on_settings_close_behavior_changed)

        self.check_updates_btn.clicked.connect(self.check_for_updates)

        self.home_preset_combo.currentIndexChanged.connect(self.on_home_preset_changed)

        self.home_zapret_start_check.checkedChanged.connect(self.on_home_service_toggled)

        self.home_tg_start_check.checkedChanged.connect(self.on_home_service_toggled)

        self.tg_port_input.valueChanged.connect(self.on_tg_settings_changed)

        self.settings_tg_port_input.valueChanged.connect(self.on_tg_settings_changed)

        self.tg_secret_input.editingFinished.connect(self.on_tg_settings_changed)

        self.settings_tg_secret_input.editingFinished.connect(self.on_tg_settings_changed)

        self.activity_zapret_btn.clicked.connect(lambda: self.switch_activity_console(0))

        self.activity_tg_btn.clicked.connect(lambda: self.switch_activity_console(1))

        self.filter_all_btn.clicked.connect(lambda: self.apply_filter("all"))

        self.filter_passed_btn.clicked.connect(lambda: self.apply_filter("passed"))

        self.filter_working_btn.clicked.connect(lambda: self.apply_filter("working"))

        self.filter_favorites_btn.clicked.connect(lambda: self.apply_filter("favorites"))

        self.relaunch_admin_btn.clicked.connect(self.relaunch_as_admin)

        self.table_model.dataChanged.connect(self.on_table_data_changed)

        self.table.clicked.connect(self.on_table_clicked)

        self.table.doubleClicked.connect(self.on_table_double_clicked)

        self.table.selectionModel().selectionChanged.connect(self.on_selection_changed)

        self.table.viewport().installEventFilter(self)

        self.side_run_btn.clicked.connect(self.run_selected)

        self.side_copy_path_btn.clicked.connect(self.copy_selected_path)

        self.side_open_file_btn.clicked.connect(self.open_selected_file)

        self.side_reveal_file_btn.clicked.connect(self.reveal_selected_file)

        self.side_test_details_btn.clicked.connect(self.show_selected_test_details)

        self.side_delete_btn.clicked.connect(self.delete_or_hide_selected)

        self.search_input.textChanged.connect(lambda _text: self.schedule_filter(self.active_filter))



    def setup_tray(self) -> None:

        if not QSystemTrayIcon.isSystemTrayAvailable():

            self.tray_icon = None

            return



        icon = self.tray_icon_for_windows()

        self.tray_icon = QSystemTrayIcon(icon, self)

        menu = QMenu(self)

        self.tray_show_action = QAction(self.tr("Show Z2 GUI"), self)

        self.tray_stop_action = QAction(self.tr("Stop winws2"), self)

        self.tray_exit_action = QAction(self.tr("Close"), self)



        self.tray_show_action.triggered.connect(self.show_from_tray)

        self.tray_stop_action.triggered.connect(self.stop_from_tray)

        self.tray_exit_action.triggered.connect(self.exit_from_tray)

        menu.aboutToShow.connect(self.update_tray_texts)



        menu.addAction(self.tray_show_action)

        menu.addAction(self.tray_stop_action)

        menu.addSeparator()

        menu.addAction(self.tray_exit_action)

        self.tray_icon.setContextMenu(menu)

        self.tray_icon.activated.connect(self.on_tray_activated)

        self.update_tray_texts()

        self.update_tray_tooltip()

        self.tray_icon.show()



    def update_tray_texts(self) -> None:

        if hasattr(self, "tray_show_action"):

            self.tray_show_action.setText(self.tr("Show Z2 GUI"))

        if hasattr(self, "tray_stop_action"):

            self.tray_stop_action.setText(self.tr("Stop winws2"))

            self.tray_stop_action.setEnabled(

                self.tests_running

                or self.winws_running

                or self.tg_ws_running

                or runner.is_winws2_running()

            )

        if hasattr(self, "tray_exit_action"):

            self.tray_exit_action.setText(self.tr("Close"))

        self.update_tray_tooltip()



    def update_tray_tooltip(self) -> None:

        tray = getattr(self, "tray_icon", None)

        if tray is not None:

            tray.setToolTip(f"Z2 GUI - {self.running_label.text() or self.tr('Idle')}")



    def on_tray_activated(self, reason: QSystemTrayIcon.ActivationReason) -> None:

        if reason in (QSystemTrayIcon.Trigger, QSystemTrayIcon.DoubleClick):

            self.show_from_tray()



    def show_from_tray(self) -> None:

        self.showNormal()

        self.activateWindow()

        self.raise_()



    def stop_from_tray(self) -> None:

        self.winws_running = self.winws_running or runner.is_winws2_running()

        self.tg_ws_running = tg_ws.is_running(

            self.settings.tg_ws_host,

            self.settings.tg_ws_port,

            self.settings.tg_ws_pid,

        )

        self.show_from_tray()

        self.stop()

        self.update_tray_texts()



    def exit_from_tray(self) -> None:

        self.winws_running = self.winws_running or runner.is_winws2_running()

        self.tg_ws_running = tg_ws.is_running(

            self.settings.tg_ws_host,

            self.settings.tg_ws_port,

            self.settings.tg_ws_pid,

        )

        if self.worker is not None:

            self.show_from_tray()

            self.stop()

            return

        if self.winws_running and not admin.is_admin():

            self.show_from_tray()

            self.show_message(

                "Administrator required",

                "Run the app as administrator to stop winws2 before closing.",

            )

            return

        if self.tg_ws_running or self.winws_running:

            self.show_from_tray()

            self.stop(after_success=self.finish_exit_from_tray)

            return

        self.finish_exit_from_tray()


    def finish_exit_from_tray(self) -> None:

        self.force_quit = True

        tray = getattr(self, "tray_icon", None)

        if self.close():

            if tray is not None:

                tray.hide()

            QApplication.quit()



    def changeEvent(self, event) -> None:

        if event.type() == QEvent.WindowStateChange and self.isMinimized():

            tray = getattr(self, "tray_icon", None)

            if tray is not None and tray.isVisible():

                QTimer.singleShot(0, self.hide)

        super().changeEvent(event)



    def _apply_style(self) -> None:

        setTheme(Theme.LIGHT if self.settings.theme == "light" else Theme.DARK)

        setThemeColor("#0078d4")

        self.setStyleSheet(app_qss(self.settings.theme))

        self.apply_home_service_card_palette()

        self.apply_window_chrome()


    def apply_home_service_card_palette(self) -> None:

        if not hasattr(self, "_service_cards"):

            return

        light = self.settings.theme in {"light"}

        win11 = self.settings.theme == "win11"

        for card in self._service_cards.values():

            palette = card.palette()

            if light:

                palette.setColor(QPalette.Window, QColor("#ffffff"))

                palette.setColor(QPalette.WindowText, QColor("#111827"))

                palette.setColor(QPalette.Mid, QColor("#d5dbe6"))

            elif win11:

                palette.setColor(QPalette.Window, QColor("#202020"))

                palette.setColor(QPalette.WindowText, QColor("#f3f3f3"))

                palette.setColor(QPalette.Mid, QColor("#3a3a3a"))

            else:

                palette.setColor(QPalette.Window, QColor("#05070b" if self.settings.theme == "amoled" else "#191c23"))

                palette.setColor(QPalette.WindowText, QColor("#ffffff" if self.settings.theme == "amoled" else "#f7f9fc"))

                palette.setColor(QPalette.Mid, QColor("#1c2330" if self.settings.theme == "amoled" else "#2a303b"))

            card.setPalette(palette)

            card.update()



    def apply_window_chrome(self) -> None:

        dark_title = self.settings.theme in {"win11", "dark", "amoled"}

        background = {

            "light": "#f4f6fa",

            "win11": "#202020",

            "amoled": "#000000",

        }.get(self.settings.theme, "#111318")

        title_text = "#f3f3f3" if dark_title else "#111827"



        self.setMicaEffectEnabled(self.settings.theme == "win11")

        self.setCustomBackgroundColor(QColor(background), QColor(background))

        self.titleBar.titleLabel.setStyleSheet(f"color: {title_text}; background: transparent;")

        self.titleBar.iconLabel.setFixedSize(36, 36)

        self.titleBar.iconLabel.setStyleSheet("background: transparent;")

        if not self.windowIcon().isNull():

            self.titleBar.iconLabel.setPixmap(self.windowIcon().pixmap(32, 32))



        if os.name != "nt":

            return

        try:

            hwnd = int(self.winId())

            value = ctypes.c_int(1 if dark_title else 0)

            ctypes.windll.dwmapi.DwmSetWindowAttribute(hwnd, 20, ctypes.byref(value), ctypes.sizeof(value))

            backdrop = ctypes.c_int(2 if self.settings.theme == "win11" else 0)

            ctypes.windll.dwmapi.DwmSetWindowAttribute(hwnd, 38, ctypes.byref(backdrop), ctypes.sizeof(backdrop))

            corners = ctypes.c_int(2)

            ctypes.windll.dwmapi.DwmSetWindowAttribute(hwnd, 33, ctypes.byref(corners), ctypes.sizeof(corners))

        except (AttributeError, OSError, ValueError):

            return



    def animate_current_page(self, _index: int) -> None:

        page = self.stackedWidget.currentWidget()

        if page is None:

            return



        QTimer.singleShot(0, lambda widget=page: self.fade_in_page(widget))



    def fade_in_page(self, page: QWidget) -> None:

        if page is not self.stackedWidget.currentWidget():

            return



        old_effect = page.graphicsEffect()

        if old_effect is not None:

            page.setGraphicsEffect(None)



        effect = QGraphicsOpacityEffect(page)

        page.setGraphicsEffect(effect)

        animation = QPropertyAnimation(effect, b"opacity", self)

        animation.setDuration(180)

        animation.setStartValue(0.25)

        animation.setEndValue(1.0)

        animation.setEasingCurve(QEasingCurve.OutCubic)

        animation.finished.connect(lambda widget=page: widget.setGraphicsEffect(None))

        self.page_animation = animation

        animation.start()



    def _update_admin_label(self) -> None:

        self.admin_label.setObjectName("AdminBadge")

        if admin.is_admin():

            self.admin_label.setText(self.tr("Admin"))

            background, border, color = self.admin_badge_colors(True)

            self.relaunch_admin_btn.setVisible(False)

        else:

            self.admin_label.setText(self.tr("Not admin"))

            background, border, color = self.admin_badge_colors(False)

            self.relaunch_admin_btn.setVisible(True)

        self.admin_label.setStyleSheet(

            f"QLabel#AdminBadge {{ background: {background}; border: 1px solid {border}; color: {color}; }}"

        )



    def admin_badge_colors(self, elevated: bool) -> tuple[str, str, str]:

        if elevated:

            if self.settings.theme == "light":

                return "#e8f6ee", "#9bd2ad", "#176b35"

            if self.settings.theme == "amoled":

                return "#071b0f", "#2e8051", "#9df0bd"

            return "#173321", "#3f7b52", "#b9f0c5"

        if self.settings.theme == "light":

            return "#fff4d6", "#e7c75f", "#7a5b00"

        if self.settings.theme == "amoled":

            return "#1f1705", "#6f5520", "#ffd98a"

        return "#3a3321", "#8b7437", "#ffe3a3"



    def update_test_settings_button(self) -> None:

        target_list = targets.load_targets(self.paths.targets_file)

        selected = self.settings.selected_targets

        total_services = len({target.service for target in target_list})

        selected_services = self.selected_service_names(target_list, selected)

        if not selected or len(selected_services) >= total_services:

            self.test_settings_btn.setText(self.tr("Test Settings: All"))

            self.testing_settings_btn.setText(self.tr("Test Settings: All"))

            self.home_targets_label.setText(f"{self.tr('Targets')}: {self.tr('All')} ({total_services})")

            self.testing_targets_label.setText(f"{self.tr('Targets')}: {self.tr('All')} ({total_services})")

        else:

            service_count = len(selected_services)

            self.test_settings_btn.setText(f"{self.tr('Test Settings')}: {service_count}")

            self.testing_settings_btn.setText(f"{self.tr('Test Settings')}: {service_count}")

            self.home_targets_label.setText(f"{self.tr('Targets')}: {service_count} / {total_services}")

            self.testing_targets_label.setText(f"{self.tr('Targets')}: {service_count} / {total_services}")



    def selected_service_names(self, target_list: list[targets.Target], selected_names: set[str]) -> set[str]:

        if not selected_names:

            return {target.service for target in target_list}

        return {target.service for target in target_list if target.name in selected_names}



    def enrich_result_details_with_targets(self) -> None:

        target_map = {target.name: target for target in targets.load_targets(self.paths.targets_file)}

        for details in self.result_details.values():

            for detail in details:

                target = target_map.get(str(detail.get("name") or ""))

                if target is None:

                    continue

                detail["category"] = detail.get("category") or target.category

                detail["service"] = detail.get("service") or target.service

                detail["primary"] = target.primary



    def recalculate_results_from_details(self, save: bool = False) -> None:

        changed = False

        for preset_name, details in self.result_details.items():

            if not details:

                continue

            result = self.result_tuple_from_details(details)

            if self.results.get(preset_name) != result:

                self.results[preset_name] = result

                changed = True

        if save and changed:

            stored_results.save_results_with_details(self.paths.gui_results, self.results, self.result_details)



    def result_tuple_from_details(self, details: stored_results.TargetDetails) -> stored_results.ResultTuple:

        ok = fail = unsup = ping_ok = ping_total = 0

        for detail in details:

            tokens = detail.get("http_tokens", [])

            if isinstance(tokens, list):

                for token in tokens:

                    text = str(token)

                    if re.search(r":(?:OK|HTTP)\d{3}$", text):

                        ok += 1

                    elif text.endswith(":UNSUP"):

                        unsup += 1

                    else:

                        fail += 1



            if detail.get("ping_target"):

                ping_total += 1

                if detail.get("ping_ok"):

                    ping_ok += 1



        score = self.score_from_details(ok, fail, ping_ok, details)

        return ok, fail, unsup, ping_ok, ping_total, score



    def score_from_details(

        self,

        ok: int,

        fail: int,

        ping_ok: int,

        details: stored_results.TargetDetails,

    ) -> int:

        score = scoring.score_result(ok, fail, ping_ok)

        passed, partial, failed = self.service_summary_counts(details)

        primary_failed = sum(

            1 for detail in details if self.detail_is_primary(detail) and not self.detail_passed(detail)

        )

        return score + passed * 100 + partial * 25 - failed * 80 - primary_failed * 120



    def _update_running_label(self) -> None:

        self.running_label.setObjectName("RunningBadge")

        if self.tests_running:

            if self.test_stop_requested:

                self.set_badge(self.tr("Stopping test..."), "testing")

            else:

                self.apply_testing_badge_style()

            return

        if self.winws_running and self.current_preset:

            self.set_badge(f"{self.tr('Running')}: {self.current_preset}", "running")

        elif self.tg_ws_running:

            self.set_badge(self.tr("Running: tg-ws"), "running")

        elif not admin.is_admin():

            self.set_badge(self.tr("Needs admin"), "warning")

        else:

            self.set_badge(self.tr("Stopped"), "stopped")

        self.update_home_summary()



    def update_home_summary(self) -> None:

        self.tg_ws_running = tg_ws.is_running(

            self.settings.tg_ws_host,

            self.settings.tg_ws_port,

            self.settings.tg_ws_pid,

        )

        elevated = admin.is_admin()

        if self.tests_running:

            state_text = self.tr("Testing")

            state_kind = "testing"

        elif self.winws_running or self.tg_ws_running:

            state_text = self.tr("Running")

            state_kind = "running"

        elif not elevated:

            state_text = self.tr("Needs admin")

            state_kind = "warning"

        else:

            state_text = self.tr("Stopped")

            state_kind = "stopped"



        active_text = self.current_preset if self.winws_running and self.current_preset else "-"

        best_name = self.best_preset_name() or "-"

        selected_preset = self.selected_preset()

        selected_name = selected_preset.name if selected_preset else "-"

        selected_is_favorite = bool(selected_preset and selected_preset.name in self.settings.favorites)

        tested = sum(1 for name in self.results if any(preset.name == name for preset in self.presets))

        self.set_home_state_badge(state_text, state_kind)

        self.home_active_label.setText(f"{self.tr('Active preset')}: {active_text}")

        self.set_service_status(

            self.home_zapret_status_label,

            self.winws_running,

            "_home_zapret_running",

        )

        zapret_mode = self.tr("included in Start") if self.settings.start_zapret_enabled else self.tr("manual only")

        zapret_state = self.tr("Running") if self.winws_running else self.tr("Stopped")

        self.home_zapret_detail_label.setText(f"● {zapret_state} · {self.tr('Active preset')}: {active_text} · {zapret_mode}")

        self.set_service_status(self.home_tg_status_label, self.tg_ws_running, "_home_tg_running")

        tg_mode = self.tr("included in Start") if self.settings.start_tg_ws_enabled else self.tr("manual only")

        tg_status = f"{self.settings.tg_ws_host}:{self.settings.tg_ws_port}"

        tg_state = self.tr("Running") if self.tg_ws_running else self.tr("Stopped")

        self.home_tg_detail_label.setText(f"● {tg_state} · {tg_status} · {tg_mode}")

        self.home_selected_label.setText(f"{self.tr('Selected preset')}: {selected_name}")

        self.home_best_label.setText(f"{self.tr('Best preset')}: {best_name}")

        self.home_presets_label.setText(f"{self.tr('Presets')}: {len(self.presets)}")

        self.home_favorites_label.setText(f"{self.tr('Favorites')}: {len(self.settings.favorites)}")

        self.home_results_label.setText(f"{self.tr('Tested')}: {tested}")

        self.testing_selected_label.setText(f"{self.tr('Selected presets')}: {len(self.checked_presets())}")

        self.testing_best_label.setText(f"{self.tr('Best result')}: {best_name}")

        self.update_readiness_summary(elevated, selected_name, selected_is_favorite)

        self.update_home_preset_combo()

        self.update_onboarding_steps(tested)

        self.update_tg_summary()

        self.update_settings_summary()


    def update_readiness_summary(self, elevated: bool, selected_name: str, selected_is_favorite: bool) -> None:

        self.readiness_admin_label.setText(self.tr("Ready") if elevated else self.tr("Run as administrator to start zapret"))

        if selected_is_favorite:

            self.readiness_preset_label.setText(selected_name)

        elif self.settings.favorites:

            self.readiness_preset_label.setText(self.tr("Choose a favorite preset"))

        else:

            self.readiness_preset_label.setText(self.tr("Add a preset to Favorites"))

        self.readiness_targets_label.setText(self.home_targets_label.text())

        enabled_services: list[str] = []

        if self.settings.start_zapret_enabled:

            enabled_services.append("zapret")

        if self.settings.start_tg_ws_enabled:

            enabled_services.append("tg-ws")

        self.readiness_services_label.setText(", ".join(enabled_services) if enabled_services else self.tr("Select at least one service"))



    def update_onboarding_steps(self, tested: int) -> None:

        self.set_onboarding_step(self.onboarding_test_label, tested > 0)

        self.set_onboarding_step(self.onboarding_favorite_label, bool(self.settings.favorites))

        self.set_onboarding_step(self.onboarding_start_label, bool(self.settings.favorites and self.selected_preset()))



    def set_onboarding_step(self, label: QLabel, done: bool) -> None:

        step_number = str(label.property("stepNumber") or label.text())

        label.setText("РІСљвЂњ" if done else step_number)

        label.setProperty("done", done)

        label.setObjectName("OnboardingStepDone" if done else "OnboardingStepNumber")

        label.style().unpolish(label)

        label.style().polish(label)



    def set_home_state_badge(self, text: str, kind: str) -> None:

        self.home_state_label.setObjectName("HomeStateBadge")

        if kind == "testing":

            background, border, color = self.testing_badge_colors()

        else:

            background, border, color = self.badge_colors(kind)

        self.home_state_label.setText(text)

        self.home_state_label.setStyleSheet(

            f"QLabel#HomeStateBadge {{ background: {background}; border: 1px solid {border}; color: {color}; }}"

        )



    def set_service_status(self, label: QLabel, running: bool, state_attr: str) -> None:

        old_value = getattr(self, state_attr)

        setattr(self, state_attr, running)

        label.setObjectName("HomeServiceStatusOn" if running else "HomeServiceStatusOff")

        label.setPixmap(QPixmap())

        label.setText(self.tr("Running") if running else self.tr("Stopped"))

        label.style().unpolish(label)

        label.style().polish(label)

        card = self._service_cards.get(label)

        if card is not None:

            card.set_running(running, animate=old_value != running)

        if old_value != running:

            self.animate_service_status(label)



    def service_status_pixmap(self, running: bool) -> QPixmap | None:

        if running in self._status_pixmaps:

            return self._status_pixmaps[running]

        candidates = ["good.png", "true.png"] if running else ["false.png"]

        for name in candidates:

            path = self.asset_path(name)

            if not path.exists():

                continue

            pixmap = QPixmap(str(path))

            if not pixmap.isNull():

                cached = pixmap.scaled(96, 96, Qt.KeepAspectRatio, Qt.SmoothTransformation)

                self._status_pixmaps[running] = cached

                return cached

        self._status_pixmaps[running] = None

        return None



    def animate_service_status(self, label: QLabel) -> None:

        old_effect = label.graphicsEffect()

        if old_effect is not None:

            label.setGraphicsEffect(None)

        effect = QGraphicsOpacityEffect(label)

        label.setGraphicsEffect(effect)

        animation = QPropertyAnimation(effect, b"opacity", self)

        animation.setDuration(220)

        animation.setStartValue(0.25)

        animation.setEndValue(1.0)

        animation.setEasingCurve(QEasingCurve.OutCubic)

        animation.finished.connect(lambda widget=label: widget.setGraphicsEffect(None))

        self.status_animations[label] = animation

        animation.start()



    def update_settings_summary(self) -> None:

        winws_text = self.tr("Running") if self.winws_running else self.tr("Stopped")

        tg_text = self.tr("Running") if self.tg_ws_running else self.tr("Stopped")

        self.settings_status_label.setText(f"winws2: {winws_text}")

        self.settings_counts_label.setText(

            f"{self.tr('Presets')}: {len(self.presets)} | {self.tr('Favorites')}: {len(self.settings.favorites)} | "

            f"{self.tr('Tested')}: {len(self.results)} | tg-ws: {tg_text}"

        )

        self.settings_paths_label.setText(f"{self.tr('Root')}: {self.paths.root}")



    def update_home_preset_combo(self) -> None:

        current = self.selected_preset()

        current_name = current.name if current and current.name in self.settings.favorites else ""

        if not current_name and self.current_preset and self.current_preset in self.settings.favorites:

            current_name = self.current_preset



        names = [

            preset.name

            for preset in sorted(self.presets, key=lambda item: item.name.casefold())

            if preset.name in self.settings.favorites

        ]

        existing = [self.home_preset_combo.itemText(index) for index in range(self.home_preset_combo.count())]

        has_favorites = bool(names)

        self.home_preset_combo.setEnabled(has_favorites)

        self.home_settings_btn.setText(self.tr("Pick Test Targets") if has_favorites else self.tr("Open Favorites"))



        empty_item_text = self.tr("Add presets to Favorites first")

        existing_sources = [self.source_text(text) for text in existing]

        if existing != names and (existing_sources != ["Add presets to Favorites first"] or has_favorites):

            self.home_preset_combo.blockSignals(True)

            self.home_preset_combo.clear()

            for name in names:

                self.home_preset_combo.addItem(name)

            if not names:

                self.home_preset_combo.addItem(empty_item_text)

            self.home_preset_combo.blockSignals(False)



        if not has_favorites:

            self.home_selected_label.setText(self.tr("No favorite presets yet. Star one or more presets on the Presets page."))

            return



        if not current_name:

            current_name = self.home_preset_combo.currentText() or names[0]



        if current_name:

            index = self.home_preset_combo.findText(current_name)

            if index >= 0 and self.home_preset_combo.currentIndex() != index:

                self.home_preset_combo.blockSignals(True)

                self.home_preset_combo.setCurrentIndex(index)

                self.home_preset_combo.blockSignals(False)

            selected = self.selected_preset()

            if selected is None or selected.name != current_name:

                self.select_preset_by_name(current_name, notify=False)



    def clear_layout(self, layout) -> None:

        while layout.count():

            item = layout.takeAt(0)

            child_layout = item.layout()

            widget = item.widget()

            if child_layout is not None:

                self.clear_layout(child_layout)

            if widget is not None:

                widget.deleteLater()



    def start_testing_badge(self) -> None:

        self.testing_badge_tick = 0

        self.apply_testing_badge_style()

        self.update_testing_badge()

        self.testing_badge_timer.start()



    def stop_testing_badge(self) -> None:

        self.testing_badge_timer.stop()

        self.testing_badge_tick = 0

        self._update_running_label()



    def update_testing_badge(self) -> None:

        if self.test_stop_requested:

            self.set_badge(self.tr("Stopping test..."), "testing")

            return

        dots = "." * (self.testing_badge_tick % 4)

        self.running_label.setText(f"{self.tr('Testing')}{dots}")

        self.update_tray_tooltip()

        self.testing_badge_tick += 1



    def testing_badge_colors(self) -> tuple[str, str, str]:

        if self.settings.theme == "light":

            return "#e1f1f7", "#75a9c3", "#154860"

        if self.settings.theme == "win11":

            return "#243847", "#3a80ad", "#c9edff"

        if self.settings.theme == "amoled":

            return "#071920", "#246f8b", "#9ee8ff"

        return "#243b45", "#4b839b", "#b9eefc"



    def apply_testing_badge_style(self) -> None:

        background, border, color = self.testing_badge_colors()

        self.running_label.setStyleSheet(

            f"QLabel#RunningBadge {{ background: {background}; border: 1px solid {border}; color: {color}; }}"

        )

        self.update_tray_tooltip()



    def set_badge(self, text: str, kind: str) -> None:

        self.running_label.setText(text)

        if kind == "testing":

            self.apply_testing_badge_style()

            self.running_label.setText(text)

        else:

            background, border, color = self.badge_colors(kind)

            self.running_label.setStyleSheet(

                f"QLabel#RunningBadge {{ background: {background}; border: 1px solid {border}; color: {color}; }}"

            )

        self.update_tray_tooltip()



    def badge_colors(self, kind: str) -> tuple[str, str, str]:

        palettes = {

            "light": {

                "running": ("#e6f6ec", "#95d2a8", "#176b35"),

                "warning": ("#fff4d6", "#e7c75f", "#7a5b00"),

                "stopped": ("#fde7e9", "#efb3ba", "#8a2531"),

            },

            "win11": {

                "running": ("#16351f", "#3f8f55", "#b8f6c4"),

                "warning": ("#3b321c", "#9a7a21", "#ffe39b"),

                "stopped": ("#3a242b", "#7a4654", "#ffd7dd"),

            },

            "amoled": {

                "running": ("#071b0f", "#2e8051", "#9df0bd"),

                "warning": ("#1f1705", "#6f5520", "#ffd98a"),

                "stopped": ("#210912", "#6f2b3e", "#ffb6c5"),

            },

            "dark": {

                "running": ("#173321", "#3f7b52", "#b9f0c5"),

                "warning": ("#3a3321", "#8b7437", "#ffe3a3"),

                "stopped": ("#341f27", "#704454", "#ffbdc6"),

            },

        }

        theme_colors = palettes.get(self.settings.theme, palettes["dark"])

        return theme_colors.get(kind, theme_colors["stopped"])



    def reload_presets(self) -> None:

        self.begin_busy("Loading presets...")

        try:

            self.current_preset = state.read_current_preset(self.paths.current_preset)

            self.winws_running = runner.is_winws2_running()

            self.tg_ws_running = tg_ws.is_running(

                self.settings.tg_ws_host,

                self.settings.tg_ws_port,

                self.settings.tg_ws_pid,

            )

            self._update_running_label()

            self.update_test_settings_button()

            self.presets = [

                preset

                for preset in presets.list_presets(self.paths.presets_dir)

                if preset.name not in self.settings.hidden_presets

            ]

            rows: list[PresetTableRow] = []

            for preset in self.presets:

                row = PresetTableRow(preset=preset, favorite=preset.name in self.settings.favorites)

                status = "Active" if self.is_active_preset(preset.name) else "Ready"

                row.values[3] = status

                row.sort_values[0] = 1 if row.favorite else 0

                row.sort_values[3] = status

                if preset.name in self.results:

                    self.populate_result_row(row, preset.name, self.results[preset.name])

                rows.append(row)

            self.table_model.set_theme(self.settings.theme, self.status_colors())

            self.table_model.set_rows(rows)

            self.schedule_filter(self.active_filter, immediate=True)

            self.log_message(f"Loaded presets: {len(self.presets)}")

            self.update_home_summary()

            self.update_action_states()

        finally:

            self.end_busy()



    def is_active_preset(self, preset_name: str) -> bool:

        return self.winws_running and preset_name == self.current_preset



    def populate_result_row(

        self,

        row: PresetTableRow,

        preset_name: str,

        result: stored_results.ResultTuple,

    ) -> None:

        ok, fail, unsup, ping_ok, ping_total, score = result

        status = "Active" if self.is_active_preset(preset_name) else self.result_status_for(preset_name)

        service_summary, service_sort = self.service_summary_for(preset_name)

        values: dict[int, tuple[str, int | str]] = {

            3: (status, status),

            4: (str(ok), ok),

            5: (str(fail), fail),

            6: (str(unsup), unsup),

            7: (f"{ping_ok}/{ping_total}", ping_ok),

            8: (service_summary, service_sort),

            9: (str(score), score),

        }

        for column, (text, sort_value) in values.items():

            row.values[column] = text

            row.sort_values[column] = sort_value



    def show_preset_actions(self, preset_name: str, source: QWidget | None = None, global_pos=None) -> None:

        preset = self.preset_by_name(preset_name)

        if preset is None:

            self.notify_warning("Preset not found", preset_name)

            return



        menu = QMenu(self.table)

        favorite_action = menu.addAction("Remove from Favorites" if preset.name in self.settings.favorites else "Add to Favorites")

        favorite_action.setIcon(FluentIcon.HEART.qicon())

        menu.addSeparator()

        copy_action = menu.addAction("Copy Path")

        copy_action.setIcon(FluentIcon.COPY.qicon())

        open_action = menu.addAction("Open File")

        open_action.setIcon(FluentIcon.DOCUMENT.qicon())

        reveal_action = menu.addAction("Reveal in Folder")

        reveal_action.setIcon(FluentIcon.FOLDER.qicon())

        details_action = menu.addAction("Test Details")

        details_action.setIcon(FluentIcon.INFO.qicon())

        menu.addSeparator()

        delete_action = menu.addAction("Delete/Hide")

        delete_action.setIcon(FluentIcon.DELETE.qicon())



        details_action.setEnabled(preset.name in self.result_details)

        delete_action.setEnabled(not self.tests_running)



        if global_pos is None:

            source = source or self.row_action_widget(preset.name) or self.table

            global_pos = source.mapToGlobal(source.rect().bottomLeft())



        chosen = menu.exec(global_pos)

        if chosen == favorite_action:

            row = self.row_for_preset(preset.name)

            if row >= 0:

                self.toggle_favorite_row(row, preset)

        elif chosen == copy_action:

            self.copy_preset_path(preset)

        elif chosen == open_action:

            self.open_preset_file(preset)

        elif chosen == reveal_action:

            self.reveal_preset_file(preset)

        elif chosen == details_action:

            self.show_preset_test_details(preset.name)

        elif chosen == delete_action:

            self.delete_or_hide_preset(preset)



    def row_action_widget(self, preset_name: str) -> QWidget | None:

        source_row = self.row_for_preset(preset_name)

        proxy_row = self.proxy_row_for_source(source_row)

        if proxy_row < 0:

            return None

        return self.table.viewport()



    def preset_by_name(self, preset_name: str) -> presets.Preset | None:

        for preset in self.presets:

            if preset.name == preset_name:

                return preset

        return None



    def selected_preset(self) -> presets.Preset | None:

        selected = self.table.selectionModel().selectedRows()

        if not selected:

            return None

        source_index = self.table_proxy.mapToSource(selected[0])

        return self.table_model.preset_at(source_index.row())



    def selected_is_active_preset(self) -> bool:

        preset = self.selected_preset()

        return bool(preset and self.winws_running and preset.name == self.current_preset)



    def selected_row(self) -> int:

        selected = self.table.selectionModel().selectedRows()

        if not selected:

            return -1

        return self.table_proxy.mapToSource(selected[0]).row()



    def select_preset_by_name(self, preset_name: str, notify: bool = True) -> None:

        source_row = self.row_for_preset(preset_name)

        proxy_row = self.proxy_row_for_source(source_row)

        if proxy_row < 0:

            return

        self.table.selectRow(proxy_row)

        self.update_home_summary()

        if notify:

            self.notify_info("Preset selected", preset_name)



    def on_home_preset_changed(self) -> None:

        preset_name = self.home_preset_combo.currentText()

        if preset_name and preset_name != "Add presets to Favorites first":

            self.select_preset_by_name(preset_name, notify=False)



    def on_home_service_toggled(self, *_args) -> None:

        self.settings.start_zapret_enabled = self.home_zapret_start_check.isChecked()

        self.settings.start_tg_ws_enabled = self.home_tg_start_check.isChecked()

        settings.save_settings(self.paths.gui_settings, self.settings)

        self.update_home_summary()

        self.update_action_states()



    def toggle_home_power(self) -> None:

        if self.tests_running:

            self.stop()

            return

        if self.winws_running or self.tg_ws_running:

            self.stop()

            return

        if not self.settings.start_zapret_enabled and not self.settings.start_tg_ws_enabled:

            self.show_message("Nothing selected", "Enable at least one Home service card before pressing Start.")

            return

        preset = self.selected_preset()

        if self.settings.start_zapret_enabled and (preset is None or preset.name not in self.settings.favorites):

            self.show_message("Favorites required", "Add a preset to Favorites first.")

            return

        if self.settings.start_zapret_enabled and not admin.is_admin():

            self.show_message("Administrator required", "Run the app as administrator to start winws2.")

            return

        if self.settings.start_tg_ws_enabled and not self.tg_ws_running:

            self.start_tg_ws_proxy(after_success=lambda: self.run_selected() if self.settings.start_zapret_enabled else self.finish_home_start_without_zapret())

            return

        if self.settings.start_zapret_enabled:

            self.run_selected()

        else:

            self.update_home_summary()

            self.update_action_states()


    def finish_home_start_without_zapret(self) -> None:

        self.update_home_summary()

        self.update_action_states()



    def start_tg_ws_proxy(self, after_success=None) -> bool:

        if self.tg_ws_running:

            if after_success is not None:

                after_success()

            return True

        params = {

            "paths": self.paths,

            "host": self.settings.tg_ws_host,

            "port": self.settings.tg_ws_port,

            "secret": self.settings.tg_ws_secret,

        }

        def finished(_op: str, result: object) -> None:

            data = dict(result)

            self.settings.tg_ws_pid = int(data.get("pid") or 0)

            settings.save_settings(self.paths.gui_settings, self.settings)

            self.tg_ws_running = bool(data.get("running"))

            if not self.tg_ws_running:

                self.show_message("tg-ws exited", "tg-ws proxy exited immediately. Check utils/tg-ws-proxy.log.")

                self.log_message("tg-ws exited immediately.")

                self.log_tg_message("tg-ws exited immediately. Check utils/tg-ws-proxy.log.")

                self.update_home_summary()

                self.update_action_states()

                return

            self.log_message(f"Started tg-ws: {self.settings.tg_ws_host}:{self.settings.tg_ws_port}")

            self.log_tg_message(f"Started tg-ws: {self.settings.tg_ws_host}:{self.settings.tg_ws_port}")

            self.load_tg_ws_log_tail()

            self.notify_success("tg-ws started", tg_ws.proxy_link(self.settings.tg_ws_host, self.settings.tg_ws_port, self.settings.tg_ws_secret))

            self.update_home_summary()

            self.update_action_states()

            if after_success is not None:

                after_success()

        def failed(_op: str, message: str) -> None:

            self.show_message("Cannot start tg-ws", message)

        return self.start_service_operation("start_tg", "Starting tg-ws...", params, finished, failed)



    def stop_tg_ws_proxy(self, after_success=None) -> bool:

        params = {

            "pid": self.settings.tg_ws_pid,

            "host": self.settings.tg_ws_host,

            "port": self.settings.tg_ws_port,

        }

        def finished(_op: str, _result: object) -> None:

            self.settings.tg_ws_pid = 0

            settings.save_settings(self.paths.gui_settings, self.settings)

            self.tg_ws_running = False

            self.log_message("Stopped tg-ws.")

            self.log_tg_message("Stopped tg-ws.")

            self.update_home_summary()

            self.update_action_states()

            if after_success is not None:

                after_success()

        return self.start_service_operation("stop_tg", "Stopping tg-ws...", params, finished)



    def tg_proxy_link(self) -> str:

        return tg_ws.proxy_link(self.settings.tg_ws_host, self.settings.tg_ws_port, self.settings.tg_ws_secret)


    def start_service_operation(self, operation: str, busy_text: str, params: dict[str, object], on_finished, on_failed=None) -> bool:

        if self.service_thread is not None and self.service_thread.isRunning():

            self.notify_info("Operation is running", "Wait for the current service operation to finish.")

            return False

        self.begin_busy(busy_text)

        thread = QThread(self)

        worker = ServiceOperationWorker(operation, params)

        worker.moveToThread(thread)

        self.service_thread = thread

        self.service_worker = worker

        self.service_pending_result = None

        self.service_finished_callback = on_finished

        self.service_failed_callback = on_failed

        thread.started.connect(worker.run)

        worker.finished.connect(self.on_service_operation_finished)

        worker.failed.connect(self.on_service_operation_failed)

        worker.finished.connect(worker.deleteLater)

        worker.failed.connect(worker.deleteLater)

        worker.finished.connect(thread.quit)

        worker.failed.connect(thread.quit)

        thread.finished.connect(self.on_service_thread_finished)

        thread.finished.connect(thread.deleteLater)

        thread.start()

        self.update_action_states()

        return True


    def on_service_operation_finished(self, op: str, result: object) -> None:

        callback = self.service_finished_callback

        self.service_finished_callback = None

        self.service_failed_callback = None

        self.service_pending_result = ("finished", callback, op, result)

        self.end_busy()


    def on_service_operation_failed(self, op: str, message: str) -> None:

        callback = self.service_failed_callback

        self.service_finished_callback = None

        self.service_failed_callback = None

        self.service_pending_result = ("failed", callback, op, message)

        self.end_busy()


    def on_service_thread_finished(self) -> None:

        if self.sender() is not self.service_thread:

            return

        self.service_thread = None

        self.service_worker = None

        self.service_finished_callback = None

        self.service_failed_callback = None

        self.update_action_states()

        pending = self.service_pending_result

        self.service_pending_result = None

        if pending is not None:

            QTimer.singleShot(0, lambda pending=pending: self.finish_service_operation_callback(pending))


    def finish_service_operation_callback(self, pending) -> None:

        status, callback, op, payload = pending

        if callback is not None:

            callback(op, payload)

        elif status == "failed":

            self.show_message("Service operation failed", payload)



    def update_tg_summary(self) -> None:

        self.tg_ws_running = tg_ws.is_running(

            self.settings.tg_ws_host,

            self.settings.tg_ws_port,

            self.settings.tg_ws_pid,

        )

        status = self.tr("Running") if self.tg_ws_running else self.tr("Stopped")

        endpoint = f"{self.settings.tg_ws_host}:{self.settings.tg_ws_port}"

        kind = "running" if self.tg_ws_running else "stopped"

        background, border, color = self.badge_colors(kind)

        for label in (self.tg_status_label, self.settings_tg_status_label):

            label.setText(status)

            label.setStyleSheet(

                f"QLabel#TgStatusBadge {{ background: {background}; border: 1px solid {border}; color: {color}; }}"

            )

        for label in (self.tg_endpoint_label, self.settings_tg_endpoint_label):

            label.setText(endpoint)

        link_text = f"{self.tr('Proxy link')}: {self.tg_proxy_link()}"

        for label in (self.tg_link_label, self.settings_tg_link_label):

            label.setText(link_text)



    def sync_tg_controls(self) -> None:

        for spin in (self.tg_port_input, self.settings_tg_port_input):

            if spin.value() != self.settings.tg_ws_port:

                spin.blockSignals(True)

                spin.setValue(self.settings.tg_ws_port)

                spin.blockSignals(False)

        for edit in (self.tg_secret_input, self.settings_tg_secret_input):

            if edit.text() != self.settings.tg_ws_secret:

                edit.blockSignals(True)

                edit.setText(self.settings.tg_ws_secret)

                edit.blockSignals(False)

        self.update_tg_summary()



    def on_tg_settings_changed(self, *_args) -> None:

        sender = self.sender()

        port = self.settings_tg_port_input.value() if sender is self.settings_tg_port_input else self.tg_port_input.value()

        if sender in (self.tg_secret_input, self.settings_tg_secret_input):

            secret = sender.text().strip()

        else:

            secret = self.settings.tg_ws_secret

        if len(secret) != 32:

            self.notify_warning("Invalid tg-ws secret", "Secret must be 32 hex characters.")

            self.sync_tg_controls()

            return

        try:

            bytes.fromhex(secret)

        except ValueError:

            self.notify_warning("Invalid tg-ws secret", "Secret must contain only hex characters.")

            self.sync_tg_controls()

            return



        changed_running = self.tg_ws_running and (

            port != self.settings.tg_ws_port or secret != self.settings.tg_ws_secret

        )

        self.settings.tg_ws_port = port

        self.settings.tg_ws_secret = secret

        settings.save_settings(self.paths.gui_settings, self.settings)

        self.sync_tg_controls()

        if changed_running:

            self.notify_info("tg-ws settings saved", "Restart tg-ws to apply the new port or secret.")



    def copy_tg_proxy_link(self) -> None:

        QApplication.clipboard().setText(self.tg_proxy_link())

        self.notify_success("Proxy link copied", self.tg_proxy_link())



    def open_tg_proxy_link(self) -> None:

        if not QDesktopServices.openUrl(QUrl(self.tg_proxy_link())):

            self.copy_tg_proxy_link()

            self.show_message("Open in Telegram", "Could not open Telegram automatically. Proxy link was copied.")



    def restart_tg_ws_proxy(self) -> None:

        if self.tg_ws_running:

            self.stop_tg_ws_proxy(after_success=lambda: self.start_tg_ws_proxy(after_success=self.after_tg_restart_started))

            return

        self.start_tg_ws_proxy(after_success=self.after_tg_restart_started)


    def after_tg_restart_started(self) -> None:

        self.update_home_summary()

        self.update_action_states()



    def open_tg_ws_log(self) -> None:

        path = tg_ws.log_file(self.paths)

        if not path.exists():

            path.parent.mkdir(parents=True, exist_ok=True)

            path.write_text("", encoding="utf-8")

        self.open_local_file(path, "tg-ws log")



    def load_tg_ws_log_tail(self) -> None:

        path = tg_ws.log_file(self.paths)

        if not path.exists():

            return

        try:

            lines = path.read_text(encoding="utf-8", errors="replace").splitlines()

        except OSError as exc:

            self.log_tg_message(f"Could not read tg-ws log: {exc}")

            return

        self.tg_activity_log.clear()

        for line in lines[-160:]:

            self.log_tg_message(line)



    def home_secondary_action(self) -> None:

        if self.settings.favorites:

            self.open_test_settings()

            return

        self.switchTo(self.presets_interface)

        self.apply_filter("all")

        self.notify_info("Favorites", "Use the star column to add presets to Home.")



    def add_preset(self) -> None:

        filename, _ = QFileDialog.getOpenFileName(

            self,

            "Add preset",

            str(self.paths.root),

            "Preset files (*.txt)",

        )

        if not filename:

            return



        try:

            preset = presets.add_preset(self.paths.presets_dir, Path(filename))

        except Exception as exc:

            self.show_message("Cannot add preset", str(exc))

            return



        self.log_message(f"Added preset: {preset.name}")

        self.notify_success("Preset added", preset.name)

        self.settings.custom_presets.add(preset.name)

        settings.save_settings(self.paths.gui_settings, self.settings)

        self.reload_presets()



    def open_presets_folder(self) -> None:

        self.paths.presets_dir.mkdir(parents=True, exist_ok=True)

        os.startfile(self.paths.presets_dir)

        self.notify_info("Presets folder opened", str(self.paths.presets_dir))



    def open_local_file(self, path: Path, label: str) -> None:

        if not path.exists():

            path.parent.mkdir(parents=True, exist_ok=True)

            if path == self.paths.gui_settings:

                settings.save_settings(self.paths.gui_settings, self.settings)

            elif path == self.paths.gui_results:

                stored_results.save_results_with_details(self.paths.gui_results, self.results, self.result_details)

        try:

            os.startfile(path)

        except OSError as exc:

            self.show_message(label, str(exc))

            return

        self.notify_info(f"{label} opened", str(path))



    def maybe_show_onboarding(self) -> None:

        if self.settings.onboarding_seen:

            return

        self.show_onboarding_dialog(force=False)


    def onboarding_tour_steps(self) -> list[dict]:

        return [

            {

                "title": "Welcome to Z2 GUI",

                "body": "Z2 GUI helps you test zapret presets, save working ones, launch them from Home, and manage the optional tg-ws Telegram proxy. It is not a VPN.",

                "icon": FluentIcon.HOME,

                "page": self.home_interface,

                "target": lambda: self.home_readiness_card,

            },

            {

                "title": "Administrator rights",

                "body": "Starting zapret requires administrator rights because winws2 and WinDivert work with network packets. Browsing presets and configuring tg-ws does not require elevation.",

                "icon": FluentIcon.PEOPLE,

                "page": self.home_interface,

                "target": lambda: self.home_readiness_admin_item,

            },

            {

                "title": "Presets page",

                "body": "The Presets page contains all available configurations. Use the Test column to choose what to check, and use the star column to add good presets to Favorites.",

                "icon": FluentIcon.APPLICATION,

                "page": self.presets_interface,

                "target": lambda: self.table,

                "max_width": 620,

                "max_height": 280,

            },

            {

                "title": "Test settings",

                "body": "Open Test Settings to choose services and endpoints. Fewer targets make checks faster when you only need Discord, YouTube, Telegram, or another specific service.",

                "icon": FluentIcon.SETTING,

                "page": self.presets_interface,

                "target": lambda: self.test_settings_btn,

            },

            {

                "title": "Results and score",

                "body": "After testing, look at Status, Services, Ping, and Score. Higher score usually means a better candidate, but real network behavior can still vary.",

                "icon": FluentIcon.SPEED_HIGH,

                "page": self.presets_interface,

                "target": lambda: self.table,

                "align": "right",

                "max_width": 560,

                "max_height": 280,

            },

            {

                "title": "Home launch",

                "body": "Home is the control panel. Choose a favorite preset, enable zapret and/or tg-ws with the switches, then press Start.",

                "icon": FluentIcon.PLAY,

                "page": self.home_interface,

                "target": lambda: self.home_power_btn,

            },

            {

                "title": "tg-ws proxy",

                "body": "The TG page manages the local Telegram MTProto WebSocket proxy. You can set the port and secret, restart it, copy the link, or open it in Telegram.",

                "icon": FluentIcon.SEND,

                "page": self.tg_interface,

                "target": lambda: self.tg_port_input,

            },

            {

                "title": "Activity logs",

                "body": "Activity shows separate logs for zapret and tg-ws. When something fails, this page is the first place to inspect what happened.",

                "icon": FluentIcon.HISTORY,

                "page": self.activity_interface,

                "target": lambda: self.activity_stack,

            },

            {

                "title": "You are ready",

                "body": "A good first run is: open Presets, select a few configs, test them, star the best result, return Home, and press Start.",

                "icon": FluentIcon.ACCEPT,

                "page": self.home_interface,

                "target": lambda: self.home_power_btn,

            },

        ]



    def show_onboarding_dialog(self, force: bool = False) -> None:

        steps = self.onboarding_tour_steps()

        if not steps:

            return

        existing = getattr(self, "_onboarding_overlay", None)

        if existing is not None:

            existing.close()

        def mark_seen() -> None:

            self.settings.onboarding_seen = True

            settings.save_settings(self.paths.gui_settings, self.settings)

        self._onboarding_overlay = OnboardingOverlay(self, steps, mark_seen)

        self._onboarding_overlay.start(0)

        return

        steps = [

            {

                "title": "Welcome to Z2 GUI",

                "body": "Z2 GUI helps you test zapret presets, save working ones, launch them from Home, and manage the optional tg-ws Telegram proxy. It is not a VPN.",

                "icon": FluentIcon.HOME,

                "page": self.home_interface,

            },

            {

                "title": "Administrator rights",

                "body": "Starting zapret requires administrator rights because winws2 and WinDivert work with network packets. Browsing presets and configuring tg-ws does not require elevation.",

                "icon": FluentIcon.PEOPLE,

                "page": self.home_interface,

            },

            {

                "title": "Presets page",

                "body": "The Presets page contains all available configurations. Use the Test column to choose what to check, and use the star column to add good presets to Favorites.",

                "icon": FluentIcon.APPLICATION,

                "page": self.presets_interface,

            },

            {

                "title": "Test settings",

                "body": "Open Test Settings to choose services and endpoints. Fewer targets make checks faster when you only need Discord, YouTube, Telegram, or another specific service.",

                "icon": FluentIcon.SETTING,

                "page": self.presets_interface,

            },

            {

                "title": "Results and score",

                "body": "After testing, look at Status, Services, Ping, and Score. Higher score usually means a better candidate, but real network behavior can still vary.",

                "icon": FluentIcon.SPEED_HIGH,

                "page": self.presets_interface,

            },

            {

                "title": "Home launch",

                "body": "Home is the control panel. Choose a favorite preset, enable zapret and/or tg-ws with the switches, then press Start.",

                "icon": FluentIcon.PLAY,

                "page": self.home_interface,

            },

            {

                "title": "tg-ws proxy",

                "body": "The TG page manages the local Telegram MTProto WebSocket proxy. You can set the port and secret, restart it, copy the link, or open it in Telegram.",

                "icon": FluentIcon.SEND,

                "page": self.tg_interface,

            },

            {

                "title": "Activity logs",

                "body": "Activity shows separate logs for zapret and tg-ws. When something fails, this page is the first place to inspect what happened.",

                "icon": FluentIcon.HISTORY,

                "page": self.activity_interface,

            },

            {

                "title": "You are ready",

                "body": "A good first run is: open Presets, select a few configs, test them, star the best result, return Home, and press Start.",

                "icon": FluentIcon.ACCEPT,

                "page": self.home_interface,

            },

        ]

        dialog = QDialog(self)

        dialog.setWindowTitle("Welcome to Z2 GUI")

        dialog.setModal(True)

        dialog.setMinimumWidth(620)

        dialog.setStyleSheet(app_qss(self.settings.theme))

        layout = QVBoxLayout(dialog)

        layout.setContentsMargins(22, 22, 22, 18)

        layout.setSpacing(16)

        header = QHBoxLayout()

        icon_label = QLabel()

        icon_label.setObjectName("TourIcon")

        icon_label.setAlignment(Qt.AlignCenter)

        header.addWidget(icon_label)

        title_box = QVBoxLayout()

        title_box.setContentsMargins(0, 0, 0, 0)

        title_label = QLabel()

        title_label.setObjectName("Title")

        progress_label = QLabel()

        progress_label.setObjectName("MetaLabel")

        title_box.addWidget(title_label)

        title_box.addWidget(progress_label)

        header.addLayout(title_box, 1)

        layout.addLayout(header)

        body_label = QLabel()

        body_label.setObjectName("TourBody")

        body_label.setWordWrap(True)

        layout.addWidget(body_label)

        hint = QLabel(self.tr("You can replay this guide later from Settings / Tools."))

        hint.setObjectName("MetaLabel")

        hint.setWordWrap(True)

        layout.addWidget(hint)

        actions = QHBoxLayout()

        skip_btn = QPushButton("Skip")

        back_btn = QPushButton("Back")

        next_btn = PrimaryButton("Next")

        actions.addWidget(skip_btn)

        actions.addStretch(1)

        actions.addWidget(back_btn)

        actions.addWidget(next_btn)

        layout.addLayout(actions)

        state = {"index": 0}

        def mark_seen() -> None:

            self.settings.onboarding_seen = True

            settings.save_settings(self.paths.gui_settings, self.settings)

        def show_step(index: int) -> None:

            state["index"] = max(0, min(index, len(steps) - 1))

            step = steps[state["index"]]

            page = step.get("page")

            if isinstance(page, QWidget):

                self.switchTo(page)

            icon = step["icon"]

            icon_label.setPixmap(icon.qicon().pixmap(34, 34))

            title_label.setText(self.tr(str(step["title"])))

            body_label.setText(self.tr(str(step["body"])))

            progress_label.setText(f"{state['index'] + 1} / {len(steps)}")

            back_btn.setEnabled(state["index"] > 0)

            skip_btn.setText(self.tr("Skip"))

            back_btn.setText(self.tr("Back"))

            next_btn.setText(self.tr("Finish") if state["index"] == len(steps) - 1 else self.tr("Next"))

        def skip() -> None:

            mark_seen()

            dialog.reject()

        def back() -> None:

            show_step(state["index"] - 1)

        def next_step() -> None:

            if state["index"] >= len(steps) - 1:

                mark_seen()

                dialog.accept()

                return

            show_step(state["index"] + 1)

        skip_btn.clicked.connect(skip)

        back_btn.clicked.connect(back)

        next_btn.clicked.connect(next_step)

        show_step(0)

        dialog.exec()



    def export_settings_backup(self) -> None:

        self.paths.utils_dir.mkdir(parents=True, exist_ok=True)

        default_name = self.paths.root / f"z2-gui-settings-backup-{time.strftime('%Y%m%d-%H%M%S')}.zip"

        filename, _ = QFileDialog.getSaveFileName(

            self,

            "Export settings",

            str(default_name),

            "Z2 GUI backup (*.zip)",

        )

        if not filename:

            return

        backup_path = Path(filename)

        if backup_path.suffix.lower() != ".zip":

            backup_path = backup_path.with_suffix(".zip")



        settings.save_settings(self.paths.gui_settings, self.settings)

        stored_results.save_results_with_details(self.paths.gui_results, self.results, self.result_details)



        try:

            with zipfile.ZipFile(backup_path, "w", compression=zipfile.ZIP_DEFLATED) as archive:

                if self.paths.gui_settings.exists():

                    archive.write(self.paths.gui_settings, "utils/gui-settings.json")

                if self.paths.gui_results.exists():

                    archive.write(self.paths.gui_results, "utils/gui-results.json")

                custom_names = set(self.settings.custom_presets)

                for preset in self.presets:

                    if preset.name in custom_names and preset.path.exists():

                        archive.write(preset.path, f"presets/{preset.path.name}")

                notice = self.paths.root / "docs" / "NOTICE.md"

                if notice.exists():

                    archive.write(notice, "docs/NOTICE.md")

        except OSError as exc:

            self.show_message("Export Settings", str(exc))

            return



        self.log_message(f"Exported settings backup: {backup_path}")

        self.notify_success("Settings exported", str(backup_path))



    def import_settings_backup(self) -> None:

        filename, _ = QFileDialog.getOpenFileName(

            self,

            "Import settings",

            str(self.paths.root),

            "Z2 GUI backup (*.zip);;Settings JSON (*.json)",

        )

        if not filename:

            return



        source = Path(filename)

        message = QMessageBox(self)

        message.setWindowTitle("Import Settings")

        message.setText(

            "Importing settings will replace current local settings and last test results.\n\n"

            "Current running processes will not be imported. Continue?"

        )

        import_button = message.addButton("Import", QMessageBox.AcceptRole)

        message.addButton("Cancel", QMessageBox.RejectRole)

        message.exec()

        if message.clickedButton() is not import_button:

            return



        try:

            self._import_settings_source(source)

        except Exception as exc:

            self.show_message("Import Settings", str(exc))

            return



        self.settings = settings.load_settings(self.paths.gui_settings)

        self.settings.tg_ws_pid = 0

        settings.save_settings(self.paths.gui_settings, self.settings)

        self.results = stored_results.load_results(self.paths.gui_results)

        self.result_details = stored_results.load_result_details(self.paths.gui_results)

        self.enrich_result_details_with_targets()

        self.recalculate_results_from_details(save=True)

        self.sync_settings_controls()

        self._apply_style()

        self.update_test_settings_button()

        self.update_tg_status()

        self.reload_presets()

        self.log_message(f"Imported settings backup: {source}")

        self.notify_success("Settings imported", "Settings, results, and custom presets were restored.")



    def check_for_updates(self) -> None:

        if self.update_thread is not None and self.update_thread.isRunning():

            self.notify_info("Update check", "Update check is already running.")

            return



        self.check_updates_btn.setEnabled(False)

        self.check_updates_btn.setText(self.tr("Checking..."))

        self.log_message("Checking for updates...")



        self.update_thread = QThread(self)

        self.update_worker = UpdateCheckWorker(APP_VERSION)

        self.update_worker.moveToThread(self.update_thread)

        self.update_pending_result = None

        self.update_thread.started.connect(self.update_worker.run)

        self.update_worker.finished.connect(self.on_update_check_finished)

        self.update_worker.failed.connect(self.on_update_check_failed)

        self.update_worker.finished.connect(self.update_worker.deleteLater)

        self.update_worker.failed.connect(self.update_worker.deleteLater)

        self.update_worker.finished.connect(self.update_thread.quit)

        self.update_worker.failed.connect(self.update_thread.quit)

        self.update_thread.finished.connect(self.on_update_thread_finished)

        self.update_thread.finished.connect(self.update_thread.deleteLater)

        self.update_thread.start()



    def on_update_check_finished(self, info: UpdateInfo) -> None:

        self.log_message(f"Update check finished: current={info.current_version}, latest={info.latest_tag}")

        self.update_pending_result = ("finished", info)


    def finish_update_check(self, pending) -> None:

        status, payload = pending

        if status == "failed":

            self.log_message(f"Update check failed: {payload}")

            self.show_message("Update check failed", payload)

            return

        info = payload

        if info.is_newer:

            self.show_update_dialog(info)

        else:

            self.show_message(

                "No updates",

                f"Z2 GUI {APP_VERSION} is already the latest available version.",

            )



    def on_update_check_failed(self, message: str) -> None:

        self.update_pending_result = ("failed", message)



    def on_update_thread_finished(self) -> None:

        self.check_updates_btn.setEnabled(True)

        self.check_updates_btn.setText(self.tr("Check for Updates"))

        self.update_worker = None

        self.update_thread = None

        pending = self.update_pending_result

        self.update_pending_result = None

        if pending is not None:

            QTimer.singleShot(0, lambda pending=pending: self.finish_update_check(pending))



    def show_update_dialog(self, info: UpdateInfo) -> None:

        dialog = QDialog(self)

        dialog.setWindowTitle(self.tr("Update Available"))

        dialog.resize(760, 580)



        layout = QVBoxLayout(dialog)

        layout.setContentsMargins(22, 22, 22, 22)

        layout.setSpacing(14)



        title = QLabel(f"Z2 GUI {info.latest_tag} is available")

        title.setObjectName("Title")

        layout.addWidget(title)



        version_label = QLabel(f"Current version: {info.current_version} | Latest version: {info.latest_version}")

        version_label.setObjectName("MetaLabel")

        layout.addWidget(version_label)



        warning = QLabel(

            "Before updating, export your settings. Local settings and test results are stored in "

            "utils/gui-settings.json and utils/gui-results.json."

        )

        warning.setObjectName("MetaLabel")

        warning.setWordWrap(True)

        layout.addWidget(warning)



        changes_label = QLabel(self.tr("Changes"))

        changes_label.setObjectName("SectionTitle")

        layout.addWidget(changes_label)



        changes = QTextEdit()

        changes.setReadOnly(True)

        changes.setPlainText(info.changes)

        changes.setMinimumHeight(260)

        layout.addWidget(changes, 1)



        actions = QHBoxLayout()

        actions.addStretch(1)



        export_btn = QPushButton(self.tr("Export Settings"))

        export_btn.setIcon(FluentIcon.SAVE)

        export_btn.clicked.connect(self.export_settings_backup)

        actions.addWidget(export_btn)



        release_btn = QPushButton(self.tr("Open Release"))

        release_btn.setIcon(FluentIcon.LINK)

        release_btn.clicked.connect(lambda: QDesktopServices.openUrl(QUrl(info.release_url or PROJECT_RELEASES_URL)))

        actions.addWidget(release_btn)



        download_btn = PrimaryButton(self.tr("Download Portable"))

        download_btn.setIcon(FluentIcon.DOWNLOAD)

        download_btn.clicked.connect(

            lambda: QDesktopServices.openUrl(QUrl(info.asset_url or info.release_url or PROJECT_RELEASES_URL))

        )

        download_btn.setEnabled(bool(info.asset_url or info.release_url))

        actions.addWidget(download_btn)



        later_btn = QPushButton(self.tr("Later"))

        later_btn.clicked.connect(dialog.accept)

        actions.addWidget(later_btn)



        layout.addLayout(actions)

        dialog.exec()



    def _import_settings_source(self, source: Path) -> None:

        self.paths.utils_dir.mkdir(parents=True, exist_ok=True)

        self.paths.presets_dir.mkdir(parents=True, exist_ok=True)

        if source.suffix.lower() == ".json":

            self.paths.gui_settings.write_bytes(source.read_bytes())

            return



        with zipfile.ZipFile(source, "r") as archive:

            names = set(archive.namelist())

            if "utils/gui-settings.json" not in names:

                raise ValueError("Backup does not contain utils/gui-settings.json.")

            self.paths.gui_settings.write_bytes(archive.read("utils/gui-settings.json"))

            if "utils/gui-results.json" in names:

                self.paths.gui_results.write_bytes(archive.read("utils/gui-results.json"))

            for member in names:

                if not member.startswith("presets/") or member.endswith("/"):

                    continue

                preset_name = Path(member).name

                if not preset_name.lower().endswith(".txt"):

                    continue

                target = self.paths.presets_dir / preset_name

                target.write_bytes(archive.read(member))



    def show_message(self, title: str, content: str) -> None:

        dialog = MessageBox(title, content, self)

        dialog.exec()



    def show_selected_test_details(self) -> None:

        preset = self.selected_preset()

        if preset is None:

            self.show_message("Test Details", "Select a preset first.")

            return

        self.show_preset_test_details(preset.name)



    def show_preset_test_details(self, preset_name: str) -> None:

        details = self.result_details.get(preset_name, [])

        if not details:

            self.show_message("Test Details", "No saved target details for this preset yet.")

            return



        dialog = QDialog(self)

        dialog.setWindowTitle(f"Test Details - {preset_name}")

        dialog.resize(820, 620)



        layout = QVBoxLayout(dialog)

        layout.setContentsMargins(18, 18, 18, 18)

        layout.setSpacing(12)



        title = QLabel(preset_name)

        title.setObjectName("Title")

        title.setWordWrap(True)

        layout.addWidget(title)



        text = QTextEdit()

        text.setReadOnly(True)

        text.setPlainText(self.format_test_details(details))

        layout.addWidget(text, 1)



        close_row = QHBoxLayout()

        close_row.addStretch(1)

        close_btn = PrimaryButton("Close")

        close_btn.clicked.connect(dialog.accept)

        close_row.addWidget(close_btn)

        layout.addLayout(close_row)

        dialog.exec()



    def format_test_details(self, details: stored_results.TargetDetails) -> str:

        groups: dict[str, dict[str, list[dict[str, object]]]] = {}

        for detail in details:

            category = str(detail.get("category") or "General")

            service = str(detail.get("service") or category)

            groups.setdefault(category, {}).setdefault(service, []).append(detail)



        lines: list[str] = []

        service_states = [

            self.service_state(service_details)

            for services in groups.values()

            for service_details in services.values()

        ]

        passed_services = sum(1 for state in service_states if state == "passed")

        partial_services = sum(1 for state in service_states if state == "partial")

        lines.append(f"Services passed: {passed_services}/{len(service_states)}")

        if partial_services:

            lines.append(f"Partial services: {partial_services}")

        lines.append("")



        for category in sorted(groups, key=str.casefold):

            lines.append(f"[{category}]")

            for service in sorted(groups[category], key=str.casefold):

                service_details = groups[category][service]

                state = self.service_state(service_details).upper()

                passed = sum(1 for detail in service_details if self.detail_passed(detail))

                primary_details = [detail for detail in service_details if self.detail_is_primary(detail)]

                primary_passed = sum(1 for detail in primary_details if self.detail_passed(detail))

                if primary_details:

                    summary = f"main {primary_passed}/{len(primary_details)}, endpoints {passed}/{len(service_details)}"

                else:

                    summary = f"endpoints {passed}/{len(service_details)}"

                lines.append(f"{state:7} {service} ({summary})")

                if primary_details and primary_passed < len(primary_details):

                    lines.append("        Main endpoint failed; service is treated as FAILED.")

                for detail in service_details:

                    marker = "OK" if self.detail_passed(detail) else "FAIL"

                    role = "MAIN" if self.detail_is_primary(detail) else "diag"

                    name = str(detail.get("name") or "-")

                    url = str(detail.get("url") or detail.get("ping_target") or "-")

                    tokens = detail.get("http_tokens", [])

                    token_text = " ".join(str(token) for token in tokens) if isinstance(tokens, list) else "-"

                    ping_state = "OK" if detail.get("ping_ok") else "FAIL"

                    ping_text = str(detail.get("ping_text") or "-")

                    reason = self.detail_reason(detail)

                    lines.append(f"  {marker:4} {role:4} {name}")

                    lines.append(f"       URL: {url}")

                    lines.append(f"       HTTP: {token_text or '-'}")

                    lines.append(f"       Ping: {ping_state} ({ping_text})")

                    if reason:

                        lines.append(f"       Reason: {reason}")

            lines.append("")

        return "\n".join(lines).strip()



    def service_state(self, details: list[dict[str, object]]) -> str:

        primary_details = [detail for detail in details if self.detail_is_primary(detail)]

        if primary_details:

            primary_passed = sum(1 for detail in primary_details if self.detail_passed(detail))

            if primary_passed == len(primary_details):

                return "passed"

            return "failed"



        passed = sum(1 for detail in details if self.detail_passed(detail))

        if passed == len(details) and details:

            return "passed"

        if passed:

            return "partial"

        return "failed"



    def detail_is_primary(self, detail: dict[str, object]) -> bool:

        return bool(detail.get("primary", False))



    def detail_passed(self, detail: dict[str, object]) -> bool:

        tokens = detail.get("http_tokens", [])

        if isinstance(tokens, list) and tokens:

            return any(re.search(r":(?:OK|HTTP)\d{3}$", str(token)) for token in tokens)

        return bool(detail.get("ping_ok"))



    def detail_reason(self, detail: dict[str, object]) -> str:

        if self.detail_passed(detail):

            return ""

        tokens = detail.get("http_tokens", [])

        token_text = " ".join(str(token) for token in tokens) if isinstance(tokens, list) else ""

        if "DNS_LOOPBACK" in token_text:

            return "DNS resolves to localhost"

        if "FAIL6_DNS" in token_text or ":ERR_DNS" in token_text:

            return "DNS resolution failed"

        if "FAIL7_CONNECT" in token_text:

            return "TCP connect refused or blocked"

        if "FAIL28_TIMEOUT" in token_text or "ERR_TIMEOUT" in token_text:

            return "HTTP/TLS timeout"

        if "FAIL35_TLS" in token_text or ":SSL" in token_text:

            return "TLS/SSL handshake failed"

        if "FAIL56_RESET" in token_text:

            return "Connection reset"

        if "UNSUP" in token_text:

            return "Protocol unsupported by curl/system TLS"

        if detail.get("ping_target") and not detail.get("ping_ok"):

            return "Ping failed"

        return "Request failed"



    def copy_selected_path(self) -> None:

        preset = self.selected_preset()

        if preset is None:

            self.show_message("Copy Path", "Select a preset first.")

            return

        self.copy_preset_path(preset)



    def copy_preset_path(self, preset: presets.Preset) -> None:

        path = str(preset.path)

        QApplication.clipboard().setText(path)

        self.log_message(f"Copied path: {path}")

        self.notify_success("Path copied", path)



    def delete_or_hide_selected(self) -> None:

        preset = self.selected_preset()

        if preset is None:

            self.show_message("Delete/Hide", "Select a preset first.")

            return

        self.delete_or_hide_preset(preset)



    def delete_or_hide_preset(self, preset: presets.Preset) -> None:

        if preset.name in self.settings.custom_presets:

            message = QMessageBox(self)

            message.setWindowTitle("Custom preset")

            message.setText(f"What do you want to do with this custom preset?\n\n{preset.name}")

            delete_button = message.addButton("Delete file", QMessageBox.DestructiveRole)

            hide_button = message.addButton("Hide only", QMessageBox.AcceptRole)

            cancel_button = message.addButton(QMessageBox.Cancel)

            message.exec()



            clicked = message.clickedButton()

            if clicked is None or clicked == cancel_button:

                return



            if clicked == delete_button:

                try:

                    preset.path.unlink(missing_ok=True)

                except OSError as exc:

                    self.show_message("Delete custom preset", str(exc))

                    return

                self.settings.custom_presets.discard(preset.name)

                self.settings.favorites.discard(preset.name)

                self.results.pop(preset.name, None)

                self.result_details.pop(preset.name, None)

                stored_results.save_results_with_details(self.paths.gui_results, self.results, self.result_details)

                self.log_message(f"Deleted custom preset: {preset.name}")

            elif clicked == hide_button:

                self.settings.hidden_presets.add(preset.name)

                self.log_message(f"Hidden custom preset: {preset.name}")

        else:

            answer = QMessageBox.question(

                self,

                "Hide preset",

                f"Hide built-in preset from the table?\n\n{preset.name}",

            )

            if answer != QMessageBox.Yes:

                return

            self.settings.hidden_presets.add(preset.name)

            self.settings.favorites.discard(preset.name)

            self.log_message(f"Hidden preset: {preset.name}")



        settings.save_settings(self.paths.gui_settings, self.settings)

        self.reload_presets()



    def restore_hidden_presets(self) -> None:

        if not self.settings.hidden_presets:

            self.show_message("Restore hidden", "No hidden presets.")

            return

        count = len(self.settings.hidden_presets)

        self.settings.hidden_presets.clear()

        settings.save_settings(self.paths.gui_settings, self.settings)

        self.reload_presets()

        self.log_message(f"Restored hidden presets: {count}")



    def run_selected(self) -> None:

        preset = self.selected_preset()

        if preset is None:

            self.show_message("Run selected", "Select a preset first.")

            return



        if not admin.is_admin():

            self.show_message("Administrator required", "Run the app as administrator to start winws2.")

            return

        if self.selected_is_active_preset():

            self.log_message(f"Already running: {preset.name}")

            self.notify_info("Already running", preset.name)

            self.update_action_states()

            return

        if not self.ensure_no_conflicts("start this preset"):

            return



        params = {

            "paths": self.paths,

            "preset_path": preset.path,

            "preset_name": preset.name,

        }

        def finished(_op: str, result: object) -> None:

            data = dict(result)

            running = bool(data.get("running"))

            returncode = data.get("returncode")

            if not running:

                self.show_message(

                    "Preset exited",

                    f"winws2 exited immediately with code {returncode}. Check the preset or run from console for details.",

                )

                self.log_message(f"winws2 exited immediately: {preset.name} code={returncode}")

                self.winws_running = False

                self._update_running_label()

                self.update_active_status("")

                self.update_action_states()

                return

            self.current_preset = preset.name

            self.winws_running = True

            self._update_running_label()

            self.update_active_status(preset.name)

            self.update_action_states()

            self.log_message(f"Started preset: {preset.name}")

            self.notify_success("Preset started", preset.name)

        def failed(_op: str, message: str) -> None:

            self.show_message("Cannot run preset", message)

        self.start_service_operation("start_preset", f"Starting {preset.name}...", params, finished, failed)



    def select_all_presets(self) -> None:

        self.set_all_preset_checks(Qt.Checked)



    def clear_preset_selection(self) -> None:

        self.set_all_preset_checks(Qt.Unchecked)



    def set_all_preset_checks(self, state_: Qt.CheckState) -> None:

        self.table_model.set_all_checked(state_ == Qt.Checked)



    def select_favorites(self) -> None:

        self.table_model.set_checked_for_favorites()



    def checked_presets(self) -> list[presets.Preset]:

        return self.table_model.checked_presets()



    def stop(self, after_success=None) -> None:

        if not admin.is_admin() and (self.winws_running or runner.is_winws2_running()):

            self.show_message("Administrator required", "Run the app as administrator to stop winws2.")

            return

        if self.worker is not None:

            self.test_stop_requested = True

            self.worker.request_stop()

            self.mark_cancelled_test_rows()

            self.stop_btn.setEnabled(False)

            self.stop_btn.setText(self.tr("Stopping..."))

            self.testing_stop_btn.setEnabled(False)

            self.testing_stop_btn.setText(self.tr("Stopping..."))

            self.home_stop_btn.setEnabled(False)

            self.home_stop_btn.setText(self.tr("Stopping..."))

            self.set_progress_text(self.tr("Stopping test..."))

            self.set_badge(self.tr("Stopping test..."), "testing")

            self.log_message("Stopping test...")

            self.notify_info(self.tr("Stopping test"), self.tr("Waiting for current check to finish."))

        elif self.winws_running:

            self.set_badge(self.tr("Stopping winws2..."), "warning")

        params = {

            "stop_tg": self.tg_ws_running,

            "tg_pid": self.settings.tg_ws_pid,

            "tg_host": self.settings.tg_ws_host,

            "tg_port": self.settings.tg_ws_port,

            "stop_winws": self.winws_running or runner.is_winws2_running(),

        }

        def finished(_op: str, result: object) -> None:

            data = dict(result)

            if data.get("stopped_tg"):

                self.settings.tg_ws_pid = 0

                settings.save_settings(self.paths.gui_settings, self.settings)

                self.tg_ws_running = False

                self.log_tg_message("Stopped tg-ws.")

            self.winws_running = False

            self._update_running_label()

            self.update_active_status("")

            self.update_action_states()

            if data.get("stopped_any"):

                self.log_message("Stopped selected services.")

            self.notify_success(self.tr("Stopped"), self.tr("Selected services stopped."))

            if after_success is not None:

                after_success()

        self.start_service_operation("stop_services", "Stopping services...", params, finished)



    def closeEvent(self, event) -> None:

        if not self.force_quit and self.isVisible() and self.isMinimized():

            event.ignore()

            self.hide()

            return



        self.begin_busy("Closing...")

        self.winws_running = runner.is_winws2_running()

        self.tg_ws_running = tg_ws.is_running(

            self.settings.tg_ws_host,

            self.settings.tg_ws_port,

            self.settings.tg_ws_pid,

        )

        if self.worker is not None:

            self.test_stop_requested = True

            self.worker.request_stop()

            self.mark_cancelled_test_rows()

            self.set_badge(self.tr("Stopping test..."), "testing")

            if self.worker_thread is not None and self.worker_thread.isRunning():

                self.close_after_tests_stop = True

                self.force_quit = True

                self.end_busy()

                event.ignore()

                return



        if not self.winws_running:

            if self.tg_ws_running:

                tg_ws.stop(self.settings.tg_ws_pid, self.settings.tg_ws_host, self.settings.tg_ws_port)

                self.settings.tg_ws_pid = 0

                settings.save_settings(self.paths.gui_settings, self.settings)

                self.tg_ws_running = False

            self.flush_log_messages()

            self.end_busy()

            event.accept()

            return



        action = self.settings.close_winws_action

        if action == "ask":

            action = self.ask_close_winws_action()

            if action == "cancel":

                self.force_quit = False

                self.end_busy()

                event.ignore()

                return



        if action == "stop":

            if not admin.is_admin():

                self.show_message(

                    "Administrator required",

                    "Run the app as administrator to stop winws2 before closing.",

                )

                self.force_quit = False

                self.end_busy()

                event.ignore()

                return

            runner.stop_winws2()

            self.winws_running = False

            self.set_badge(self.tr("Stopping winws2..."), "warning")

            self.log_message("Stopped winws2 on exit.")



        if self.tg_ws_running:

            tg_ws.stop(self.settings.tg_ws_pid, self.settings.tg_ws_host, self.settings.tg_ws_port)

            self.settings.tg_ws_pid = 0

            settings.save_settings(self.paths.gui_settings, self.settings)

            self.tg_ws_running = False



        self.flush_log_messages()

        self.end_busy()

        event.accept()



    def ask_close_winws_action(self) -> str:

        dialog = QDialog(self)

        dialog.setWindowTitle("Close Z2 GUI")

        dialog.setModal(True)

        dialog.setMinimumWidth(520)

        dialog.setStyleSheet(app_qss(self.settings.theme))

        layout = QVBoxLayout(dialog)

        layout.setContentsMargins(22, 22, 22, 18)

        layout.setSpacing(16)

        header = QHBoxLayout()

        icon_label = QLabel()

        icon_label.setObjectName("CloseDialogIcon")

        icon_label.setAlignment(Qt.AlignCenter)

        icon_label.setPixmap(FluentIcon.INFO.qicon().pixmap(28, 28))

        header.addWidget(icon_label)

        text_box = QVBoxLayout()

        text_box.setContentsMargins(0, 0, 0, 0)

        text_box.setSpacing(6)

        title = QLabel(self.tr("winws2 is still running."))

        title.setObjectName("DialogTitle")

        body = QLabel(self.tr("Stop winws2 before closing the app?"))

        body.setObjectName("DialogText")

        body.setWordWrap(True)

        text_box.addWidget(title)

        text_box.addWidget(body)

        header.addLayout(text_box, 1)

        layout.addLayout(header)

        remember = QCheckBox(self.tr("Don't ask again"))

        layout.addWidget(remember)

        actions = QHBoxLayout()

        actions.setSpacing(10)

        stop_button = PrimaryButton(self.tr("Yes, stop it"))

        keep_button = QPushButton(self.tr("No, leave running"))

        cancel_button = QPushButton(self.tr("Cancel"))

        for button in (stop_button, keep_button, cancel_button):

            button.setMinimumWidth(142)

        actions.addStretch(1)

        actions.addWidget(stop_button)

        actions.addWidget(keep_button)

        actions.addWidget(cancel_button)

        layout.addLayout(actions)

        choice = {"action": "cancel"}

        def choose(action: str) -> None:

            choice["action"] = action

            dialog.accept()

        stop_button.clicked.connect(lambda: choose("stop"))

        keep_button.clicked.connect(lambda: choose("keep"))

        cancel_button.clicked.connect(dialog.reject)

        if dialog.exec() != QDialog.Accepted:

            return "cancel"

        action = choice["action"]

        if action == "cancel":

            return "cancel"

        if remember.isChecked():

            self.settings.close_winws_action = action

            settings.save_settings(self.paths.gui_settings, self.settings)

        return action



    def ensure_no_conflicts(self, action_label: str) -> bool:

        found = conflicts.find_conflicts()

        if not found:

            conflicts.flush_dns_cache()

            self.log_message("Clean test preflight: no VPN/proxy conflicts found, DNS cache flushed.")

            return True



        dialog = ConflictDialog(found, action_label, self)

        dialog.setStyleSheet(app_qss(self.settings.theme))

        dialog.exec()



        if dialog.choice == "cancel":

            self.log_message("Cancelled because possible VPN conflicts were found.")

            return False

        if dialog.choice == "continue":

            conflicts.flush_dns_cache()

            self.log_message(f"Continuing with possible conflicts: {len(found)}")

            self.notify_warning("VPN conflicts ignored", f"Processes: {len(found)}")

            return True



        if dialog.choice == "kill":

            selected = dialog.selected_processes()

            if not selected:

                self.notify_warning("No processes selected", "Select at least one process or continue anyway.")

                return False

            self.log_message(f"Killing possible VPN conflicts: {len(selected)}")

            conflicts.kill_processes(selected)

            self.wait_with_busy_animation(0.5)

            remaining = conflicts.find_conflicts()

            if not remaining:

                conflicts.flush_dns_cache()

                self.log_message("Possible VPN conflicts closed.")

                self.log_message("DNS cache flushed before testing.")

                self.notify_success("Conflicts closed", f"Processes killed: {len(selected)}")

                return True



            remaining_details = "\n".join(f"- {process.display_name}" for process in remaining)

            retry = QMessageBox(self)

            retry.setWindowTitle("Conflicts still running")

            retry.setIcon(QMessageBox.Warning)

            retry.setText("Some possible VPN/proxy processes are still running.")

            retry.setInformativeText("Continue anyway?")

            retry.setDetailedText(remaining_details)

            continue_after_fail = retry.addButton("Continue anyway", QMessageBox.AcceptRole)

            retry.addButton(QMessageBox.Cancel)

            retry.exec()

            if retry.clickedButton() == continue_after_fail:

                conflicts.flush_dns_cache()

                self.log_message(f"Continuing with remaining conflicts: {len(remaining)}")

                return True



        self.log_message("Cancelled because possible VPN conflicts are still running.")

        return False



    def test_all_placeholder(self) -> None:

        if self.tests_running or (self.worker_thread is not None and self.worker_thread.isRunning()):

            self.show_message("Test All", "Tests are already running.")

            return

        if self.winws_running:

            self.show_message("Test Selected", "Stop the running preset before testing.")

            return

        if not admin.is_admin():

            self.show_message("Administrator required", "Run the app as administrator to test presets.")

            return

        selected_presets = self.checked_presets()

        if not selected_presets:

            self.show_message("Test All", "Select at least one preset to test.")

            return

        selected_targets = self.selected_target_names_for_run()

        if not selected_targets:

            self.show_message("Test All", "Select at least one target in Test Settings.")

            return

        if not self.ensure_no_conflicts("test presets"):

            return



        self.tests_running = True

        self.test_stop_requested = False

        self.completed_this_run = 0

        for row, table_row in enumerate(self.table_model.rows):

            status = "Queued" if table_row.checked else "Skipped"

            if status == "Queued":

                self._set_row_status(row, status)

                for column in range(4, 10):

                    self.set_cell(row, column, "-", -1)

            elif table_row.preset.name not in self.results:

                self._set_row_status(row, status)



        self.worker_thread = QThread(self)

        self.worker = TestAllWorker(self.paths, selected_presets, target_names=selected_targets)

        self.worker.moveToThread(self.worker_thread)

        self.tests_pending_finish = False

        self.worker_thread.started.connect(self.worker.run)

        self.worker.preset_started.connect(self.on_preset_started)

        self.worker.target_finished.connect(self.on_target_finished)

        self.worker.preset_details_finished.connect(self.on_preset_details_finished)

        self.worker.preset_finished.connect(self.on_preset_finished)

        self.worker.log.connect(self.log_message)

        self.worker.finished.connect(self.on_tests_worker_finished)

        self.worker.finished.connect(self.worker_thread.quit)

        self.worker.finished.connect(self.worker.deleteLater)

        self.worker_thread.finished.connect(self.on_test_thread_finished)

        self.worker_thread.finished.connect(self.worker_thread.deleteLater)

        self.worker_thread.start()

        self.start_testing_badge()

        self.update_home_summary()



        self.test_all_btn.setEnabled(False)

        self.testing_test_btn.setEnabled(False)

        self.home_test_btn.setEnabled(False)

        self.test_settings_btn.setEnabled(False)

        self.testing_settings_btn.setEnabled(False)

        self.home_settings_btn.setEnabled(False)

        self.run_selected_btn.setEnabled(False)

        self.testing_run_btn.setEnabled(False)

        self.home_run_btn.setEnabled(False)

        self.add_preset_btn.setEnabled(False)

        self.open_presets_btn.setEnabled(False)

        self.remove_preset_btn.setEnabled(False)

        self.restore_hidden_btn.setEnabled(False)

        self.select_all_btn.setEnabled(False)

        self.select_favorites_btn.setEnabled(False)

        self.clear_selection_btn.setEnabled(False)

        self.refresh_btn.setEnabled(False)

        self.stop_btn.setEnabled(True)

        self.stop_btn.setText(self.tr("Cancel Test"))

        self.testing_stop_btn.setEnabled(True)

        self.testing_stop_btn.setText(self.tr("Cancel Test"))

        self.home_stop_btn.setEnabled(True)

        self.home_stop_btn.setText(self.tr("Cancel Test"))

        self.progress_bar.setRange(0, len(selected_presets))

        self.progress_bar.setValue(0)

        self.testing_progress_bar.setRange(0, len(selected_presets))

        self.testing_progress_bar.setValue(0)

        self.set_progress_text(f"Testing 0/{len(selected_presets)}")

        self.log_message(

            f"Test started. Selected presets: {len(selected_presets)} | Targets: {len(selected_targets)}"

        )



    def selected_target_names_for_run(self) -> set[str]:

        target_list = targets.load_targets(self.paths.targets_file)

        available = {target.name for target in target_list}

        if not self.settings.selected_targets:

            return available

        selected_services = self.selected_service_names(target_list, self.settings.selected_targets)

        if not selected_services:

            return self.settings.selected_targets & available

        return {target.name for target in target_list if target.service in selected_services}



    def open_test_settings(self) -> None:

        target_list = targets.load_targets(self.paths.targets_file)

        dialog = TestSettingsDialog(target_list, self.settings.selected_targets, self, language=self.settings.language)

        if dialog.exec() != TestSettingsDialog.Accepted:

            return



        self.settings.selected_targets = dialog.selected_target_names()

        settings.save_settings(self.paths.gui_settings, self.settings)

        self.update_test_settings_button()

        self.update_home_summary()

        selected_services = self.selected_service_names(target_list, self.settings.selected_targets)

        self.log_message(

            f"Test services saved: {len(selected_services)} | Endpoints: {len(self.settings.selected_targets)}"

        )

        self.notify_success("Test settings saved", f"Services: {len(selected_services)}")



    def sync_settings_controls(self) -> None:

        theme_index = self.option_index(THEME_OPTIONS, self.settings.theme)

        if theme_index >= 0 and self.settings_theme_combo.currentIndex() != theme_index:

            self.settings_theme_combo.blockSignals(True)

            self.settings_theme_combo.setCurrentIndex(theme_index)

            self.settings_theme_combo.blockSignals(False)



        language_index = self.option_index(LANGUAGE_OPTIONS, self.settings.language)

        if language_index >= 0 and self.settings_language_combo.currentIndex() != language_index:

            self.settings_language_combo.blockSignals(True)

            self.settings_language_combo.setCurrentIndex(language_index)

            self.settings_language_combo.blockSignals(False)



        close_index = self.option_index(CLOSE_OPTIONS, self.settings.close_winws_action)

        if close_index >= 0 and self.settings_close_combo.currentIndex() != close_index:

            self.settings_close_combo.blockSignals(True)

            self.settings_close_combo.setCurrentIndex(close_index)

            self.settings_close_combo.blockSignals(False)

        for checkbox, value in (

            (self.home_zapret_start_check, self.settings.start_zapret_enabled),

            (self.home_tg_start_check, self.settings.start_tg_ws_enabled),

        ):

            if checkbox.isChecked() != value:

                checkbox.blockSignals(True)

                checkbox.setChecked(value)

                checkbox.blockSignals(False)

        self.sync_tg_controls()



    def on_settings_theme_changed(self) -> None:

        theme = self.option_value(THEME_OPTIONS, self.settings_theme_combo.currentIndex())

        if not theme or theme == self.settings.theme:

            return

        self.settings.theme = theme

        settings.save_settings(self.paths.gui_settings, self.settings)

        self._apply_style()

        self._update_admin_label()

        self._update_running_label()

        self.repaint_table_rows()

        self.log_message(f"Theme changed: {theme}")

        self.notify_info("Theme changed", theme)



    def on_settings_language_changed(self) -> None:

        language = self.option_value(LANGUAGE_OPTIONS, self.settings_language_combo.currentIndex())

        if not language or language == self.settings.language:

            return

        self.settings.language = language

        settings.save_settings(self.paths.gui_settings, self.settings)

        self.apply_language()

        self.log_message(f"Language changed: {language}")

        self.notify_info(self.tr("Language"), "Р В РЎС“РЎРѓРЎРѓР С”Р С‘Р в„–" if language == "ru" else "English")



    def on_settings_close_behavior_changed(self) -> None:

        close_action = self.option_value(CLOSE_OPTIONS, self.settings_close_combo.currentIndex())

        if not close_action or close_action == self.settings.close_winws_action:

            return

        self.settings.close_winws_action = close_action

        settings.save_settings(self.paths.gui_settings, self.settings)

        self.log_message(f"Close behavior changed: {close_action}")

        self.notify_info("Close behavior changed", close_action)



    def set_cell(self, row: int, column: int, text: str, sort_value: int | str | None = None) -> None:

        self.table_model.set_value(row, column, text, sort_value)



    def service_summary_counts(self, details: stored_results.TargetDetails) -> tuple[int, int, int]:

        groups: dict[str, list[dict[str, object]]] = {}

        for detail in details:

            service = str(detail.get("service") or detail.get("category") or "General")

            groups.setdefault(service, []).append(detail)



        passed = partial = failed = 0

        for service_details in groups.values():

            state = self.service_state(service_details)

            if state == "passed":

                passed += 1

            elif state == "partial":

                partial += 1

            else:

                failed += 1

        return passed, partial, failed



    def service_summary_for(self, preset_name: str) -> tuple[str, int]:

        details = self.result_details.get(preset_name, [])

        if not details:

            return "-", -1

        passed, partial, failed = self.service_summary_counts(details)

        total = passed + partial + failed

        text = f"{passed} ok / {partial} part / {failed} fail"

        sort_value = passed * 1000 + partial * 100 - failed

        if total:

            sort_value += int((passed / total) * 99)

        return text, sort_value



    def row_for_preset(self, preset_name: str) -> int:

        return self.table_model.row_for_preset(preset_name)



    def proxy_row_for_source(self, source_row: int) -> int:

        if source_row < 0:

            return -1

        source_index = self.table_model.index(source_row, 0)

        proxy_index = self.table_proxy.mapFromSource(source_index)

        return proxy_index.row() if proxy_index.isValid() else -1



    def _set_row_status(self, row: int, status: str) -> None:

        self.set_cell(row, 3, status, status)



    def _paint_row(self, row: int, status: str) -> None:

        if 0 <= row < self.table_model.rowCount():

            self.table_model.dataChanged.emit(

                self.table_model.index(row, 0),

                self.table_model.index(row, self.table_model.columnCount() - 1),

                [Qt.BackgroundRole],

            )



    def status_colors(self) -> dict[str, QColor | None]:

        if self.settings.theme == "light":

            return {

                "Active": QColor("#dcf5e4"),

                "Running": QColor("#dbeafe"),

                "Working": QColor("#e1f1f7"),

                "Passed": QColor("#e1f6e8"),

                "Partial": QColor("#fff3d6"),

                "Failed": QColor("#ffe4eb"),

                "Queued": QColor("#eef2f8"),

                "Stopping": QColor("#f2e8f0"),

                "Skipped": QColor("#f5f7fb"),

            }

        if self.settings.theme == "win11":

            return {

                "Active": QColor("#14351f"),

                "Running": QColor("#162f43"),

                "Working": QColor("#172c38"),

                "Passed": QColor("#102b1b"),

                "Partial": QColor("#332b17"),

                "Failed": QColor("#351b22"),

                "Queued": None,

                "Stopping": QColor("#321f31"),

                "Skipped": None,

            }

        if self.settings.theme == "amoled":

            return {

                "Active": QColor("#071b0f"),

                "Running": QColor("#091827"),

                "Working": QColor("#071920"),

                "Passed": QColor("#071b0f"),

                "Partial": QColor("#1f1705"),

                "Failed": QColor("#210912"),

                "Queued": QColor("#0b0f15"),

                "Stopping": QColor("#160c15"),

                "Skipped": QColor("#05070b"),

            }

        return {

            "Active": QColor("#203b2b"),

            "Running": QColor("#26364d"),

            "Working": QColor("#243b45"),

            "Passed": QColor("#1f3b2a"),

            "Partial": QColor("#3b3321"),

            "Failed": QColor("#3a2227"),

            "Queued": QColor("#242a34"),

            "Stopping": QColor("#332435"),

            "Skipped": QColor("#1b1e25"),

        }



    def repaint_table_rows(self) -> None:

        self.table_model.set_theme(self.settings.theme, self.status_colors())



    def on_preset_started(self, preset_name: str, index: int, total: int) -> None:

        row = self.row_for_preset(preset_name)

        if row >= 0:

            self._set_row_status(row, "Working")

        self.set_progress_text(f"Testing {index}/{total}: {preset_name}")

        self.log_message(f"[{index}/{total}] Testing: {preset_name}")



    def on_target_finished(

        self,

        preset_name: str,

        target_name: str,

        target_index: int,

        target_total: int,

        tokens: str,

        ping: str,

    ) -> None:

        self.set_progress_text(f"{preset_name}: target {target_index}/{target_total} - {target_name}")

        self.log_message(

            f"  {preset_name} -> {target_name} [{target_index}/{target_total}]: {tokens} | Ping: {ping}"

        )



    def on_preset_details_finished(self, preset_name: str, details: object) -> None:

        if isinstance(details, list):

            self.result_details[preset_name] = [

                {

                    **detail,

                    "service": detail.get("service") or detail.get("category") or "General",

                    "primary": bool(detail.get("primary", False)),

                }

                for detail in details

                if isinstance(detail, dict)

            ]



    def on_preset_finished(

        self,

        preset_name: str,

        ok: int,

        fail: int,

        unsup: int,

        ping_ok: int,

        ping_total: int,

        score: int,

    ) -> None:

        row = self.row_for_preset(preset_name)

        if row < 0:

            return



        if preset_name in self.result_details:

            ok, fail, unsup, ping_ok, ping_total, score = self.result_tuple_from_details(

                self.result_details[preset_name]

            )

        status = self.result_status_for(preset_name)

        service_summary, service_sort = self.service_summary_for(preset_name)

        values = {

            3: status,

            4: str(ok),

            5: str(fail),

            6: str(unsup),

            7: f"{ping_ok}/{ping_total}",

            8: service_summary,

            9: str(score),

        }

        for column, value in values.items():

            sort_value = value

            if column in (4, 5, 6, 9):

                sort_value = int(value)

            elif column == 7:

                sort_value = ping_ok

            elif column == 8:

                sort_value = service_sort

            self.set_cell(row, column, value, sort_value)

        self._paint_row(row, status)

        self.results[preset_name] = (ok, fail, unsup, ping_ok, ping_total, score)

        stored_results.save_results_with_details(self.paths.gui_results, self.results, self.result_details)

        self.update_home_summary()

        self.completed_this_run += 1

        self.progress_bar.setValue(self.completed_this_run)

        self.testing_progress_bar.setValue(self.completed_this_run)

        self.log_message(

            f"  Done: {preset_name} | OK={ok} FAIL={fail} UNSUP={unsup} Ping={ping_ok}/{ping_total} Score={score}"

        )

        self.log_message(f"  Services: {service_summary}")

        self.update_selected_panel()



    def on_tests_worker_finished(self) -> None:

        self.tests_pending_finish = True


    def on_tests_finished(self) -> None:

        self.restore_pending_test_rows()

        best_name = self.best_preset_name()

        if best_name:

            best_row = self.row_for_preset(best_name)

            if best_row >= 0:

                proxy_row = self.proxy_row_for_source(best_row)

                if proxy_row >= 0:

                    self.table.selectRow(proxy_row)

            self.log_message(f"Best preset: {best_name}")



        self.tests_running = False

        self.test_stop_requested = False

        self.stop_testing_badge()

        self.table.sortByColumn(9, Qt.DescendingOrder)

        self.test_all_btn.setEnabled(True)

        self.testing_test_btn.setEnabled(True)

        self.home_test_btn.setEnabled(True)

        self.test_settings_btn.setEnabled(True)

        self.testing_settings_btn.setEnabled(True)

        self.home_settings_btn.setEnabled(True)

        self.run_selected_btn.setEnabled(True)

        self.testing_run_btn.setEnabled(True)

        self.home_run_btn.setEnabled(True)

        self.add_preset_btn.setEnabled(True)

        self.open_presets_btn.setEnabled(True)

        self.remove_preset_btn.setEnabled(True)

        self.restore_hidden_btn.setEnabled(True)

        self.select_all_btn.setEnabled(True)

        self.select_favorites_btn.setEnabled(True)

        self.clear_selection_btn.setEnabled(True)

        self.refresh_btn.setEnabled(True)

        self.set_progress_text("Idle")

        self.update_action_states()

        self.update_home_summary()

        self.log_message("Test All finished.")



    def on_test_thread_finished(self) -> None:

        pending = self.tests_pending_finish

        self.tests_pending_finish = False

        self.worker = None

        self.worker_thread = None

        self.update_action_states()

        if pending:

            QTimer.singleShot(0, self.on_tests_finished)

        if self.close_after_tests_stop:

            self.close_after_tests_stop = False

            QTimer.singleShot(0, self.close)



    def restore_pending_test_rows(self) -> None:

        for row, table_row in enumerate(self.table_model.rows):

            if table_row.status not in {"Queued", "Working", "Stopping"}:

                continue



            preset_name = table_row.preset.name

            if preset_name in self.results:

                self.apply_result_to_row(row, preset_name, self.results[preset_name], paint=True)

            else:

                self._set_row_status(row, "Ready")



    def mark_cancelled_test_rows(self) -> None:

        for row, table_row in enumerate(self.table_model.rows):

            preset_name = table_row.preset.name

            status = table_row.status

            if status == "Queued":

                if preset_name in self.results:

                    self.apply_result_to_row(row, preset_name, self.results[preset_name], paint=True)

                else:

                    self._set_row_status(row, "Ready")

            elif status == "Working":

                self._set_row_status(row, "Stopping")



    def apply_result_to_row(

        self,

        row: int,

        preset_name: str,

        result: stored_results.ResultTuple,

        paint: bool = False,

    ) -> None:

        ok, fail, unsup, ping_ok, ping_total, score = result

        status = "Active" if self.is_active_preset(preset_name) else self.result_status_for(preset_name)

        service_summary, service_sort = self.service_summary_for(preset_name)

        values = {

            3: status,

            4: str(ok),

            5: str(fail),

            6: str(unsup),

            7: f"{ping_ok}/{ping_total}",

            8: service_summary,

            9: str(score),

        }

        for column, value in values.items():

            sort_value = value

            if column in (4, 5, 6, 9):

                sort_value = int(value)

            elif column == 7:

                sort_value = ping_ok

            elif column == 8:

                sort_value = service_sort

            self.set_cell(row, column, value, sort_value)

        if paint:

            self._paint_row(row, status)



    def apply_filter(self, filter_name: str) -> None:

        self.schedule_filter(filter_name)



    def schedule_filter(self, filter_name: str, immediate: bool = False) -> None:

        self.active_filter = filter_name

        self._pending_filter = filter_name

        button_by_filter = {

            "all": self.filter_all_btn,

            "passed": self.filter_passed_btn,

            "working": self.filter_working_btn,

            "favorites": self.filter_favorites_btn,

        }

        button = button_by_filter.get(filter_name)

        if button is not None and not button.isChecked():

            button.setChecked(True)

        if immediate:

            self.filter_timer.stop()

            self.apply_filter_now()

        else:

            self.filter_timer.start()



    def apply_filter_now(self) -> None:

        self.table_proxy.set_filter(self._pending_filter, self.search_input.text())

        visible_count = self.table_proxy.rowCount()

        self.empty_state_label.setVisible(self.table_model.rowCount() > 0 and visible_count == 0)



    def best_preset_name(self) -> str:

        best_name = ""

        best_score = -1

        for name, (ok, _fail, _unsup, _ping_ok, _ping_total, score) in self.results.items():

            if ok <= 0:

                continue

            if score > best_score:

                best_name = name

                best_score = score

        return best_name



    def relaunch_as_admin(self) -> None:

        if admin.relaunch_as_admin(self.paths.root / "gui" / "app.py"):

            self.close()

        else:

            self.show_message("Administrator", "Could not start elevated process.")



    def log_message(self, message: str) -> None:

        self._zapret_log_buffer.append(message)

        if not self.log_flush_timer.isActive():

            self.log_flush_timer.start()



    def log_tg_message(self, message: str) -> None:

        self._tg_log_buffer.append(message)

        if not self.log_flush_timer.isActive():

            self.log_flush_timer.start()



    def flush_log_messages(self) -> None:

        if self._zapret_log_buffer:

            text = "\n".join(self._zapret_log_buffer)

            self._zapret_log_buffer.clear()

            self.log.append(text)

            self.activity_log.append(text)

        if self._tg_log_buffer:

            text = "\n".join(self._tg_log_buffer)

            self._tg_log_buffer.clear()

            self.tg_activity_log.append(text)

        if not self._zapret_log_buffer and not self._tg_log_buffer:

            self.log_flush_timer.stop()



    def notify_success(self, title: str, content: str) -> None:

        InfoBar.success(

            title=title,

            content=content,

            duration=2200,

            position=InfoBarPosition.TOP_RIGHT,

            parent=self,

        )



    def notify_info(self, title: str, content: str) -> None:

        InfoBar.info(

            title=title,

            content=content,

            duration=2200,

            position=InfoBarPosition.TOP_RIGHT,

            parent=self,

        )



    def notify_warning(self, title: str, content: str) -> None:

        InfoBar.warning(

            title=title,

            content=content,

            duration=3000,

            position=InfoBarPosition.TOP_RIGHT,

            parent=self,

        )



    def set_progress_text(self, text: str) -> None:

        self.progress_label.setText(text)

        self.testing_progress_label.setText(text)


    def eventFilter(self, watched, event) -> bool:

        if hasattr(self, "table") and watched is self.table.viewport() and event.type() == QEvent.MouseButtonPress:

            index = self.table.indexAt(event.pos())

            if index.isValid() and index.column() == 1:

                source_index = self.table_proxy.mapToSource(index)

                self.toggle_test_row(source_index.row())

                return True

        return super().eventFilter(watched, event)



    def begin_busy(self, text: str) -> None:

        self._busy_depth += 1

        self.busy_label.setText(text)

        self.busy_label.setVisible(False)

        self.busy_progress.setVisible(False)

        self.busy_overlay.show_busy(text)


    def wait_with_busy_animation(self, seconds: float) -> None:

        deadline = time.monotonic() + seconds

        while time.monotonic() < deadline:

            time.sleep(0.016)



    def end_busy(self) -> None:

        self._busy_depth = max(0, self._busy_depth - 1)

        if self._busy_depth:

            return

        self.busy_label.setVisible(False)

        self.busy_progress.setVisible(False)

        QTimer.singleShot(0, self.hide_busy_if_idle)


    def hide_busy_if_idle(self) -> None:

        if self._busy_depth == 0:

            self.busy_overlay.hide_busy()



    def on_table_data_changed(self, top_left, _bottom_right, _roles) -> None:

        if top_left.column() == 1:

            self.update_checked_count()



    def on_table_clicked(self, index) -> None:

        if not index.isValid():

            return

        source_index = self.table_proxy.mapToSource(index)

        preset = self.table_model.preset_at(source_index.row())

        if preset is None:

            return

        if index.column() == 0:

            self.toggle_favorite_row(source_index.row(), preset)

        elif index.column() == 10:

            self.show_preset_actions(preset.name, global_pos=QCursor.pos())



    def on_table_double_clicked(self, index) -> None:

        if not index.isValid():

            return

        source_index = self.table_proxy.mapToSource(index)

        preset = self.table_model.preset_at(source_index.row())

        if preset is None:

            return

        if index.column() == 2:

            self.select_preset_by_name(preset.name, notify=False)

        elif index.column() == 0:

            self.toggle_favorite_row(source_index.row(), preset)

        elif index.column() == 10:

            self.show_preset_actions(preset.name, global_pos=QCursor.pos())



    def toggle_favorite_row(self, source_row: int, preset: presets.Preset) -> None:

        if not 0 <= source_row < len(self.table_model.rows):

            return

        favorite = not self.table_model.rows[source_row].favorite

        self.table_model.set_favorite(source_row, favorite)

        if favorite:

            self.settings.favorites.add(preset.name)

        else:

            self.settings.favorites.discard(preset.name)

        settings.save_settings(self.paths.gui_settings, self.settings)

        self.schedule_filter(self.active_filter)

        self.update_home_summary()


    def update_checked_count(self) -> None:

        self.testing_selected_label.setText(f"{self.tr('Selected presets')}: {len(self.checked_presets())}")


    def toggle_test_row(self, source_row: int) -> None:

        if not 0 <= source_row < len(self.table_model.rows):

            return

        index = self.table_model.index(source_row, 1)

        checked = not self.table_model.rows[source_row].checked

        self.table_model.setData(index, Qt.Checked if checked else Qt.Unchecked, Qt.CheckStateRole)



    def on_selection_changed(self, *_args) -> None:

        self.update_selected_panel()

        self.update_action_states()



    def update_selected_panel(self) -> None:

        return



    def update_preset_details(self, path: Path) -> None:

        if not path.exists():

            self.selected_file_info.setText("File: missing")

            self.preset_preview.setPlainText("")

            return



        try:

            stat = path.stat()

            size_text = self.format_bytes(stat.st_size)

            modified = time.strftime("%Y-%m-%d %H:%M", time.localtime(stat.st_mtime))

            self.selected_file_info.setText(f"File: {size_text} | Modified: {modified}")

            lines = path.read_text(encoding="utf-8", errors="replace").splitlines()

        except OSError as exc:

            self.selected_file_info.setText(f"File: {exc}")

            self.preset_preview.setPlainText("")

            return



        preview = "\n".join(lines[:80])

        if len(lines) > 80:

            preview += f"\n\n... {len(lines) - 80} more lines"

        self.preset_preview.setPlainText(preview)



    def format_bytes(self, size: int) -> str:

        value = float(size)

        for unit in ("B", "KB", "MB"):

            if value < 1024 or unit == "MB":

                return f"{value:.1f} {unit}" if unit != "B" else f"{int(value)} B"

            value /= 1024

        return f"{size} B"



    def open_selected_file(self) -> None:

        preset = self.selected_preset()

        if preset is None:

            self.show_message("Open File", "Select a preset first.")

            return

        self.open_preset_file(preset)



    def open_preset_file(self, preset: presets.Preset) -> None:

        os.startfile(preset.path)

        self.notify_info("Preset file opened", preset.name)



    def reveal_selected_file(self) -> None:

        preset = self.selected_preset()

        if preset is None:

            self.show_message("Reveal in Folder", "Select a preset first.")

            return

        self.reveal_preset_file(preset)



    def reveal_preset_file(self, preset: presets.Preset) -> None:

        path = preset.path.resolve()

        if path.exists():

            result = ctypes.windll.shell32.ShellExecuteW(

                None,

                "open",

                "explorer.exe",

                f'/select,"{path}"',

                None,

                1,

            )

            if result <= 32:

                os.startfile(path.parent)

        else:

            subprocess.Popen(["explorer.exe", str(path.parent)])

        self.notify_info("Revealed in folder", preset.name)



    def update_active_status(self, active_name: str) -> None:

        for row, table_row in enumerate(self.table_model.rows):

            current_status = table_row.status

            preset_name = table_row.preset.name

            is_active_row = bool(active_name and preset_name == active_name)

            if is_active_row:

                self._set_row_status(row, "Active")

                continue

            if current_status == "Active":

                self._set_row_status(row, self.result_status_for(preset_name))

                continue

            if current_status in {"Passed", "Partial", "Failed", "Queued", "Skipped", "Working", "Stopping"}:

                continue

            status = "Ready"

            self._set_row_status(row, status)



    def update_action_states(self) -> None:

        tests_running = self.tests_running

        service_running = self.service_thread is not None and self.service_thread.isRunning()

        active_selected = self.selected_is_active_preset()

        selected = self.selected_preset()

        has_selection = selected is not None

        home_has_selection = bool(selected and selected.name in self.settings.favorites)

        home_has_service = self.settings.start_zapret_enabled or self.settings.start_tg_ws_enabled

        home_can_start_zapret = self.settings.start_zapret_enabled and home_has_selection and not active_selected

        home_can_start_tg = self.settings.start_tg_ws_enabled and not self.settings.start_zapret_enabled



        self.test_all_btn.setEnabled((not tests_running) and (not service_running) and (not self.winws_running))

        self.testing_test_btn.setEnabled((not tests_running) and (not service_running) and (not self.winws_running))

        self.home_test_btn.setEnabled((not tests_running) and (not service_running) and (not self.winws_running))

        self.run_selected_btn.setEnabled((not tests_running) and (not service_running) and has_selection and (not active_selected))

        self.testing_run_btn.setEnabled((not tests_running) and (not service_running) and has_selection and (not active_selected))

        self.home_run_btn.setEnabled((not tests_running) and (not service_running) and has_selection and (not active_selected))

        self.home_power_btn.setEnabled(

            (not service_running)

            and (

            (tests_running and not self.test_stop_requested)

            or self.winws_running

            or self.tg_ws_running

            or ((not tests_running) and (not service_running) and (home_can_start_zapret or home_can_start_tg))

            )

        )

        self.side_run_btn.setEnabled((not tests_running) and (not service_running) and has_selection and (not active_selected))

        self.side_copy_path_btn.setEnabled(has_selection)

        self.side_open_file_btn.setEnabled(has_selection)

        self.side_reveal_file_btn.setEnabled(has_selection)

        self.side_test_details_btn.setEnabled(has_selection and bool(selected and selected.name in self.result_details))

        self.side_delete_btn.setEnabled((not tests_running) and (not service_running) and has_selection)

        self.remove_preset_btn.setEnabled((not tests_running) and (not service_running) and has_selection)

        self.stop_btn.setEnabled((not service_running) and ((tests_running and not self.test_stop_requested) or self.winws_running or self.tg_ws_running))

        self.testing_stop_btn.setEnabled((not service_running) and ((tests_running and not self.test_stop_requested) or self.winws_running or self.tg_ws_running))

        self.home_stop_btn.setEnabled((not service_running) and ((tests_running and not self.test_stop_requested) or self.winws_running or self.tg_ws_running))

        self.home_settings_btn.setEnabled((not tests_running) and (not service_running))

        if not tests_running:

            self.stop_btn.setText(self.tr("Stop"))

            self.testing_stop_btn.setText(self.tr("Stop"))

            self.home_stop_btn.setText(self.tr("Stop"))

        if tests_running:

            self.home_power_btn.setText(self.tr("Stop test"))

            self.home_power_btn.setIcon(FluentIcon.CANCEL)

            self.home_power_btn.setObjectName("HomeDangerButton")

        elif self.winws_running or self.tg_ws_running:

            self.home_power_btn.setText(self.tr("Stop"))

            self.home_power_btn.setIcon(FluentIcon.CANCEL)

            self.home_power_btn.setObjectName("HomeDangerButton")

        else:

            if not home_has_service:

                button_text = self.tr("Select Service")

            elif self.settings.start_zapret_enabled and not self.settings.favorites:

                button_text = self.tr("Add Favorite")

            else:

                button_text = self.tr("Start")

            self.home_power_btn.setText(button_text)

            self.home_power_btn.setIcon(FluentIcon.PLAY)

            self.home_power_btn.setObjectName("HomePowerButton")

        if not home_has_service:

            tooltip = self.tr("Enable zapret or tg-ws on the Home cards first")

        elif self.settings.start_zapret_enabled and not self.settings.favorites:

            tooltip = self.tr("Add presets to Favorites first")

        else:

            tooltip = ""

        self.home_power_btn.setToolTip(tooltip)

        self.home_power_btn.style().unpolish(self.home_power_btn)

        self.home_power_btn.style().polish(self.home_power_btn)

        self.update_tray_texts()



    def result_status_for(self, preset_name: str) -> str:

        details = self.result_details.get(preset_name, [])

        if details:

            passed, partial, failed = self.service_summary_counts(details)

            if passed > 0 and partial == 0 and failed == 0:

                return "Passed"

            if passed > 0 or partial > 0:

                return "Partial"

            return "Failed"



        result = self.results.get(preset_name)

        if result is None:

            return "Ready"

        ok, fail, _unsup, _ping_ok, _ping_total, _score = result

        if ok > 0 and fail == 0:

            return "Passed"

        if ok > 0:

            return "Partial"

        return "Failed"

