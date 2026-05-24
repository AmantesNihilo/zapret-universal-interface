from __future__ import annotations


def app_qss(theme: str) -> str:
    if theme == "win11":
        return _build_qss(WIN11)
    if theme == "light":
        return _build_qss(LIGHT)
    if theme == "amoled":
        return _build_qss(AMOLED)
    return _build_qss(DARK)


DARK = {
    "bg": "#111318",
    "text": "#eef1f6",
    "title": "#f7f9fc",
    "section": "#c8d0dc",
    "panel": "#191c23",
    "panel_border": "#2a303b",
    "button": "#232833",
    "button_border": "#343c4a",
    "button_hover": "#2b3240",
    "button_pressed": "#1b2029",
    "nav_hover": "#232934",
    "nav_checked": "#303641",
    "disabled_bg": "#12151b",
    "disabled_border": "#202631",
    "disabled_text": "#5e6877",
    "primary": "#2f6fab",
    "primary_hover": "#377dbd",
    "primary_border": "#4483bf",
    "danger": "#3a242b",
    "danger_hover": "#4a2b35",
    "danger_border": "#704454",
    "table": "#151922",
    "table_alt": "#191f2a",
    "table_header": "#202632",
    "grid": "#29313d",
    "selection": "#2d5f95",
    "selection_text": "#ffffff",
    "log": "#0e1117",
    "meta": "#171a21",
    "running_bg": "#173321",
    "running_border": "#3f7b52",
    "running_text": "#b9f0c5",
    "stopped_bg": "#341f27",
    "stopped_border": "#704454",
    "stopped_text": "#ffbdc6",
}

LIGHT = {
    "bg": "#f4f6fa",
    "text": "#1d2430",
    "title": "#111827",
    "section": "#445065",
    "panel": "#ffffff",
    "panel_border": "#d5dbe6",
    "button": "#eef2f8",
    "button_border": "#ccd4e0",
    "button_hover": "#e4eaf3",
    "button_pressed": "#dbe3ee",
    "nav_hover": "#e9eef6",
    "nav_checked": "#dde9f7",
    "disabled_bg": "#eef1f5",
    "disabled_border": "#d9dee8",
    "disabled_text": "#9aa4b5",
    "primary": "#2869a8",
    "primary_hover": "#1f5f9d",
    "primary_border": "#2f77bb",
    "danger": "#fff0f3",
    "danger_hover": "#ffe3e9",
    "danger_border": "#e49aaa",
    "table": "#ffffff",
    "table_alt": "#f6f8fb",
    "table_header": "#eef2f8",
    "grid": "#d9e0ea",
    "selection": "#cfe4fb",
    "selection_text": "#111827",
    "log": "#ffffff",
    "meta": "#f5f7fb",
    "running_bg": "#e2f7e8",
    "running_border": "#83c89a",
    "running_text": "#1e6b35",
    "stopped_bg": "#fff0f3",
    "stopped_border": "#e49aaa",
    "stopped_text": "#8d3144",
}

AMOLED = {
    "bg": "#000000",
    "text": "#e8edf5",
    "title": "#ffffff",
    "section": "#aeb8c8",
    "panel": "#05070b",
    "panel_border": "#1c2330",
    "button": "#0d1118",
    "button_border": "#242b38",
    "button_hover": "#141a24",
    "button_pressed": "#080b10",
    "nav_hover": "#101620",
    "nav_checked": "#162032",
    "disabled_bg": "#05070b",
    "disabled_border": "#121720",
    "disabled_text": "#4f5a6b",
    "primary": "#1f6fb8",
    "primary_hover": "#267ecf",
    "primary_border": "#3388d5",
    "danger": "#210912",
    "danger_hover": "#2e0d19",
    "danger_border": "#6f2b3e",
    "table": "#030509",
    "table_alt": "#070a10",
    "table_header": "#0c1017",
    "grid": "#171d28",
    "selection": "#164a7d",
    "selection_text": "#ffffff",
    "log": "#000000",
    "meta": "#080b10",
    "running_bg": "#071b0f",
    "running_border": "#2e8051",
    "running_text": "#9df0bd",
    "stopped_bg": "#210912",
    "stopped_border": "#6f2b3e",
    "stopped_text": "#ffb6c5",
}

WIN11 = {
    "bg": "#202020",
    "text": "#f3f3f3",
    "title": "#ffffff",
    "section": "#c7c7c7",
    "panel": "#2b2b2b",
    "panel_border": "#3d3d3d",
    "button": "#323232",
    "button_border": "#454545",
    "button_hover": "#3b3b3b",
    "button_pressed": "#292929",
    "nav_hover": "#2b2b2b",
    "nav_checked": "#3a3a3a",
    "disabled_bg": "#252525",
    "disabled_border": "#303030",
    "disabled_text": "#777777",
    "primary": "#0067c0",
    "primary_hover": "#1975c5",
    "primary_border": "#2d8ae0",
    "danger": "#44272d",
    "danger_hover": "#57313a",
    "danger_border": "#8e4b5a",
    "table": "#1f1f1f",
    "table_alt": "#242424",
    "table_header": "#2b2b2b",
    "grid": "#333333",
    "selection": "#1f6aa5",
    "selection_text": "#ffffff",
    "log": "#1b1b1b",
    "meta": "#242424",
    "running_bg": "#16351f",
    "running_border": "#3f8f55",
    "running_text": "#b8f6c4",
    "stopped_bg": "#3f252d",
    "stopped_border": "#8e4b5a",
    "stopped_text": "#ffd4dc",
}


def _build_qss(c: dict[str, str]) -> str:
    return f"""
QMainWindow, QWidget {{
    background: {c["bg"]};
    color: {c["text"]};
    font-family: "Segoe UI Variable", "Segoe UI";
    font-size: 10pt;
}}

QLabel#Title {{
    font-size: 19pt;
    font-weight: 650;
    color: {c["title"]};
}}

QLabel#SectionTitle {{
    color: {c["section"]};
    font-weight: 650;
    background: transparent;
    border: none;
}}

QLabel#CardTitle {{
    color: {c["title"]};
    font-size: 12pt;
    font-weight: 700;
    background: transparent;
    border: none;
    padding: 0 0 6px 0;
}}

QLabel#PresetTitle {{
    font-size: 13pt;
    font-weight: 650;
    color: {c["title"]};
    background: transparent;
    border: none;
    padding: 0;
}}

QLabel#MetaLabel {{
    color: {c["section"]};
    background: transparent;
    border: none;
    padding: 2px 0;
}}

QLabel#TourIcon {{
    background: {c["button"]};
    border: 1px solid {c["button_border"]};
    border-radius: 22px;
    min-width: 44px;
    max-width: 44px;
    min-height: 44px;
    max-height: 44px;
}}

QLabel#TourBody {{
    color: {c["text"]};
    background: {c["meta"]};
    border: 1px solid {c["grid"]};
    border-radius: 10px;
    padding: 16px;
    font-size: 11pt;
    line-height: 145%;
}}

QFrame#TourCard {{
    background: {c["panel"]};
    border: 1px solid {c["primary_border"]};
    border-radius: 14px;
}}

QFrame#BusyPanel {{
    background: {c["panel"]};
    border: 1px solid {c["panel_border"]};
    border-radius: 10px;
}}

QWidget#BusyProgressLine {{
    background: transparent;
    border: none;
}}

QLabel#BusyTitle {{
    color: {c["title"]};
    background: transparent;
    border: none;
    font-size: 12pt;
    font-weight: 650;
}}

QLabel#TourRouteTitle {{
    color: {c["section"]};
    background: transparent;
    border: none;
    font-weight: 700;
    padding: 4px 0 0 0;
}}

QLabel#TourStepLabel {{
    color: {c["disabled_text"]};
    background: transparent;
    border: none;
    padding: 3px 0;
    font-size: 9pt;
}}

QLabel#TourStepLabel[active="true"] {{
    color: {c["primary"]};
    font-weight: 700;
}}

QLabel#InlineLabel {{
    color: {c["section"]};
    background: transparent;
    border: none;
    padding: 0 4px 0 0;
    font-weight: 500;
}}

QLabel#RunningBadge {{
    background: {c["button"]};
    border: 1px solid {c["button_border"]};
    border-radius: 8px;
    padding: 6px 11px;
    color: {c["section"]};
}}

QLabel#AdminBadge {{
    border-radius: 8px;
    padding: 6px 11px;
    font-weight: 650;
}}

QLabel#HomeStateBadge {{
    border-radius: 8px;
    padding: 7px 12px;
    font-weight: 650;
}}

QLabel#TgStatusBadge {{
    border-radius: 8px;
    padding: 7px 13px;
    font-weight: 700;
}}

QLabel#TgEndpointBadge {{
    background: {c["button"]};
    color: {c["section"]};
    border: 1px solid {c["button_border"]};
    border-radius: 8px;
    padding: 7px 12px;
    font-weight: 650;
}}

#Panel, #Sidebar {{
    background: {c["panel"]};
    border: 1px solid {c["panel_border"]};
    border-radius: 10px;
}}

#HomeServiceCard {{
    background: transparent;
    border: none;
    border-radius: 16px;
    min-height: 118px;
}}

QLabel#HomeServiceTitle {{
    color: {c["title"]};
    background: transparent;
    border: none;
    font-size: 15pt;
    font-weight: 700;
}}

QLabel#HomeServiceStatusOn {{
    color: {c["running_text"]};
    background: {c["running_bg"]};
    border: 1px solid {c["running_border"]};
    border-radius: 8px;
    padding: 6px 12px;
    font-size: 10pt;
    font-weight: 700;
}}

QLabel#HomeServiceStatusOff {{
    color: {c["stopped_text"]};
    background: {c["stopped_bg"]};
    border: 1px solid {c["stopped_border"]};
    border-radius: 8px;
    padding: 6px 12px;
    font-size: 10pt;
    font-weight: 700;
}}

QCheckBox#HomeServiceToggle {{
    background: transparent;
    border: none;
    padding: 0;
}}

QLabel#HomeSectionTitle {{
    color: {c["title"]};
    background: transparent;
    border: none;
    font-size: 15pt;
    font-weight: 700;
}}

#HomePresetCard {{
    background: {c["panel"]};
    border: 1px solid {c["panel_border"]};
    border-radius: 12px;
}}

#HomeLaunchCard {{
    background: {c["panel"]};
    border: 1px solid {c["panel_border"]};
    border-radius: 12px;
}}

QLabel#LaunchFieldLabel {{
    color: {c["section"]};
    background: transparent;
    border: none;
    font-size: 10pt;
    font-weight: 650;
}}

#OnboardingCard {{
    background: {c["panel"]};
    border: 1px solid {c["panel_border"]};
    border-radius: 12px;
}}

#ReadinessCard {{
    background: {c["panel"]};
    border: 1px solid {c["panel_border"]};
    border-radius: 12px;
}}

#ReadinessItem {{
    background: {c["meta"]};
    border: 1px solid {c["grid"]};
    border-radius: 10px;
}}

QLabel#ReadinessIcon {{
    color: {c["primary"]};
    background: {c["button"]};
    border: 1px solid {c["button_border"]};
    border-radius: 15px;
    min-width: 30px;
    max-width: 30px;
    min-height: 30px;
    max-height: 30px;
}}

#Panel[tourHighlight="true"],
#ReadinessCard[tourHighlight="true"],
#ReadinessItem[tourHighlight="true"],
#HomePresetCard[tourHighlight="true"],
#HomeLaunchCard[tourHighlight="true"],
QTableView[tourHighlight="true"],
QFrame[tourHighlight="true"],
QPushButton[tourHighlight="true"] {{
    background: {c["button_hover"]};
    border: 1px solid {c["primary_border"]};
}}

QWidget#OnboardingStep {{
    background: {c["meta"]};
    border: 1px solid {c["grid"]};
    border-radius: 10px;
}}

QLabel#OnboardingStepNumber {{
    color: {c["section"]};
    background: {c["button"]};
    border: 1px solid {c["button_border"]};
    border-radius: 15px;
    min-width: 30px;
    max-width: 30px;
    min-height: 30px;
    max-height: 30px;
    font-weight: 700;
}}

QLabel#OnboardingStepDone {{
    color: {c["running_text"]};
    background: {c["running_bg"]};
    border: 1px solid {c["running_border"]};
    border-radius: 15px;
    min-width: 30px;
    max-width: 30px;
    min-height: 30px;
    max-height: 30px;
    font-weight: 800;
}}

QLabel#OnboardingStepTitle {{
    color: {c["title"]};
    background: transparent;
    border: none;
    font-weight: 700;
}}

#CommandBar {{
    background: {c["panel"]};
    border: 1px solid {c["panel_border"]};
    border-radius: 8px;
}}

#SettingsNavBar {{
    background: transparent;
    border: none;
    border-radius: 0;
}}

QFrame#ActivitySegmentBar {{
    background: transparent;
    border: none;
}}

QPushButton#ActivitySegmentButton {{
    background: {c["button"]};
    color: {c["text"]};
    border: 1px solid {c["button_border"]};
    border-radius: 8px;
    padding: 9px 18px;
    font-weight: 650;
}}

QPushButton#ActivitySegmentButton:hover {{
    background: {c["button_hover"]};
    border-color: {c["primary_border"]};
}}

QPushButton#ActivitySegmentButton:checked {{
    background: {c["primary"]};
    color: #ffffff;
    border-color: {c["primary_border"]};
}}

QStackedWidget#ActivityStack {{
    background: transparent;
    border: none;
}}

#FluentCard {{
    background: {c["panel"]};
    border: 1px solid {c["panel_border"]};
    border-radius: 10px;
}}

QScrollArea#SidebarScroll {{
    background: transparent;
    border: none;
}}

QScrollArea#SidebarScroll > QWidget > QWidget {{
    background: transparent;
}}

QDialog {{
    background: {c["bg"]};
    color: {c["text"]};
}}

QTabWidget::pane {{
    border: 1px solid {c["panel_border"]};
    border-radius: 8px;
    background: {c["panel"]};
    top: -1px;
}}

QTabBar::tab {{
    background: transparent;
    color: {c["section"]};
    padding: 9px 18px;
    border-top-left-radius: 6px;
    border-top-right-radius: 6px;
    margin-right: 4px;
}}

QTabBar::tab:selected {{
    background: {c["primary"]};
    color: #ffffff;
    font-weight: 600;
}}

QTabBar::tab:hover:!selected {{
    background: {c["button_hover"]};
}}

QLineEdit {{
    background: {c["log"]};
    border: 1px solid {c["panel_border"]};
    border-radius: 7px;
    padding: 8px 10px;
    color: {c["text"]};
    selection-background-color: {c["selection"]};
}}

QLineEdit:focus {{
    border-color: {c["primary_border"]};
}}

QSpinBox {{
    background: {c["log"]};
    border: 1px solid {c["panel_border"]};
    border-radius: 7px;
    padding: 7px 10px;
    color: {c["text"]};
    selection-background-color: {c["selection"]};
}}

QSpinBox:focus {{
    border-color: {c["primary_border"]};
}}

QSpinBox::up-button, QSpinBox::down-button {{
    width: 18px;
    border: none;
    background: transparent;
}}

QMenu {{
    background: {c["panel"]};
    color: {c["text"]};
    border: 1px solid {c["panel_border"]};
    padding: 5px;
}}

QMenu::item {{
    padding: 7px 22px;
    border-radius: 5px;
}}

QMenu::item:selected {{
    background: {c["selection"]};
}}

QDialogButtonBox QPushButton {{
    min-width: 76px;
}}

QFrame#SettingsCard {{
    background: {c["panel"]};
    border: 1px solid {c["panel_border"]};
    border-radius: 10px;
}}

QLabel#SettingsTitle {{
    color: {c["title"]};
    font-size: 15pt;
    font-weight: 650;
}}

QLabel#SettingsSubtitle {{
    color: {c["section"]};
}}

QLabel#SettingsKey {{
    color: {c["section"]};
    font-weight: 650;
    min-width: 120px;
}}

QLabel#SettingsValue {{
    color: {c["text"]};
}}

QWidget#AboutHeader {{
    background: transparent;
    border: none;
}}

QLabel#AboutIcon {{
    background: transparent;
    border: none;
    min-width: 54px;
    min-height: 54px;
}}

QLabel#AboutTitle {{
    color: {c["title"]};
    background: transparent;
    border: none;
    font-size: 20pt;
    font-weight: 700;
}}

QLabel#AboutSubtitle {{
    color: {c["section"]};
    background: transparent;
    border: none;
    font-size: 10.5pt;
}}

QLabel#AboutVersion {{
    color: #ffffff;
    background: {c["primary"]};
    border: 1px solid {c["primary_border"]};
    border-radius: 12px;
    padding: 5px 12px;
    font-weight: 650;
    min-width: 56px;
}}

QLabel#AboutInfoKey {{
    color: {c["section"]};
    background: transparent;
    border: none;
    font-weight: 650;
    min-width: 90px;
}}

QLabel#AboutInfoValue {{
    color: {c["text"]};
    background: transparent;
    border: none;
}}

QWidget#CreditRow {{
    background: transparent;
    border: none;
}}

QLabel#CreditName {{
    color: {c["title"]};
    background: transparent;
    border: none;
    font-weight: 700;
    min-width: 190px;
}}

QLabel#CreditDetail {{
    color: {c["text"]};
    background: transparent;
    border: none;
}}

QLabel#CreditDetail a {{
    color: {c["primary"]};
    text-decoration: none;
}}

QWidget#SettingRow {{
    background: transparent;
    border: none;
}}

QWidget#SettingRow QLabel {{
    background: transparent;
    border: none;
}}

QLabel#DialogTitle {{
    color: {c["title"]};
    font-size: 14pt;
    font-weight: 650;
}}

QLabel#DialogText {{
    color: {c["section"]};
}}

QLabel#CloseDialogIcon {{
    color: {c["primary"]};
    background: {c["button"]};
    border: 1px solid {c["primary_border"]};
    border-radius: 22px;
    min-width: 44px;
    max-width: 44px;
    min-height: 44px;
    max-height: 44px;
}}

QPushButton {{
    background: {c["button"]};
    border: 1px solid {c["button_border"]};
    border-radius: 7px;
    padding: 8px 12px;
    color: {c["text"]};
    min-height: 18px;
}}

QPushButton:hover {{
    background: {c["button_hover"]};
}}

QPushButton:pressed {{
    background: {c["button_pressed"]};
}}

QPushButton:disabled {{
    color: {c["disabled_text"]};
    background: {c["disabled_bg"]};
    border-color: {c["disabled_border"]};
}}

QPushButton#PrimaryButton {{
    background: {c["primary"]};
    border-color: {c["primary_border"]};
    color: #ffffff;
    font-weight: 600;
}}

QPushButton#PrimaryButton:hover {{
    background: {c["primary_hover"]};
}}

QPushButton#PrimaryButton:disabled {{
    background: {c["disabled_bg"]};
    border-color: {c["disabled_border"]};
    color: {c["disabled_text"]};
}}

QPushButton#HomePowerButton {{
    background: {c["primary"]};
    border: 1px solid {c["primary_border"]};
    border-radius: 14px;
    color: #ffffff;
    font-size: 15pt;
    font-weight: 700;
    padding: 18px 34px;
}}

QPushButton#HomePowerButton:hover {{
    background: {c["primary_hover"]};
}}

QPushButton#HomePowerButton:disabled {{
    background: {c["disabled_bg"]};
    border-color: {c["disabled_border"]};
    color: {c["disabled_text"]};
}}

QPushButton#HomeDangerButton {{
    background: {c["danger"]};
    border: 1px solid {c["danger_border"]};
    border-radius: 14px;
    color: {c["stopped_text"]};
    font-size: 15pt;
    font-weight: 700;
    padding: 18px 34px;
}}

QPushButton#HomeDangerButton:hover {{
    background: {c["danger_hover"]};
}}

QPushButton#HomeDangerButton:disabled {{
    background: {c["disabled_bg"]};
    border-color: {c["disabled_border"]};
    color: {c["disabled_text"]};
}}

QPushButton#DangerButton {{
    background: {c["danger"]};
    border-color: {c["danger_border"]};
    color: {c["stopped_text"]};
}}

QPushButton#DangerButton:hover {{
    background: {c["danger_hover"]};
}}

QPushButton#DangerButton:disabled {{
    background: {c["disabled_bg"]};
    border-color: {c["disabled_border"]};
    color: {c["disabled_text"]};
}}

QPushButton#SegmentButton {{
    border-radius: 6px;
    padding: 6px 11px;
    background: {c["button"]};
}}

QPushButton#SegmentButton:checked {{
    background: {c["primary"]};
    border-color: {c["primary_border"]};
    color: #ffffff;
    font-weight: 600;
}}

QPushButton#SettingsNavButton {{
    background: transparent;
    border: 1px solid transparent;
    border-left: 4px solid transparent;
    border-radius: 7px;
    padding: 11px 20px 11px 16px;
    color: {c["section"]};
    font-size: 11pt;
    font-weight: 500;
    min-height: 24px;
}}

QPushButton#SettingsNavButton:hover {{
    background: {c["nav_hover"]};
    border-color: transparent;
    border-left-color: transparent;
    color: {c["text"]};
}}

QPushButton#SettingsNavButton:checked {{
    background: {c["nav_checked"]};
    border-color: {c["button_border"]};
    border-left-color: #2aa8ff;
    color: {c["text"]};
    font-weight: 650;
}}

QPushButton#SettingsNavButton:pressed {{
    background: {c["button_pressed"]};
}}

QPushButton#IconButton {{
    min-width: 34px;
    max-width: 34px;
    min-height: 34px;
    max-height: 34px;
    padding: 0px;
    border-radius: 17px;
    font-size: 14pt;
}}

QPushButton#TableActionButton {{
    min-width: 30px;
    max-width: 34px;
    min-height: 22px;
    max-height: 24px;
    padding: 0;
    border-radius: 6px;
    font-weight: 700;
}}

QTableWidget, QTableView {{
    background: {c["table"]};
    alternate-background-color: transparent;
    border: 1px solid {c["panel_border"]};
    border-radius: 8px;
    gridline-color: transparent;
    selection-background-color: {c["selection"]};
    selection-color: {c["selection_text"]};
    outline: 0;
}}

QTableWidget::item, QTableView::item {{
    padding: 8px 10px;
    border: none;
    min-height: 28px;
}}

QTableWidget::item:hover, QTableView::item:hover {{
    background: {c["button_hover"]};
    color: {c["text"]};
}}

QTableWidget::item:selected, QTableView::item:selected {{
    background: {c["selection"]};
    color: {c["selection_text"]};
}}

QHeaderView::section {{
    background: {c["table_header"]};
    color: {c["section"]};
    border: 0;
    border-bottom: 1px solid {c["grid"]};
    padding: 9px 8px;
    font-weight: 650;
}}

QTableView QTableCornerButton::section {{
    background: {c["table_header"]};
    border: 0;
    border-bottom: 1px solid {c["grid"]};
}}

QTableView::indicator {{
    width: 16px;
    height: 16px;
    border-radius: 5px;
    border: 1px solid {c["button_border"]};
    background: {c["button"]};
}}

QTableView::indicator:hover {{
    border-color: {c["primary_border"]};
    background: {c["button_hover"]};
}}

QTableView::indicator:checked {{
    background: {c["primary"]};
    border-color: {c["primary_border"]};
}}

QCheckBox {{
    color: {c["text"]};
    spacing: 7px;
}}

QCheckBox::indicator {{
    width: 16px;
    height: 16px;
    border-radius: 4px;
    border: 1px solid {c["button_border"]};
    background: {c["button"]};
}}

QCheckBox::indicator:checked {{
    background: {c["primary"]};
    border-color: {c["primary_border"]};
}}

QGroupBox {{
    border: 1px solid {c["panel_border"]};
    border-radius: 8px;
    margin-top: 12px;
    padding: 10px;
    color: {c["text"]};
}}

QGroupBox::title {{
    subcontrol-origin: margin;
    left: 10px;
    padding: 0 5px;
    color: {c["section"]};
    font-weight: 650;
}}

QTextEdit {{
    background: {c["log"]};
    border: 1px solid {c["panel_border"]};
    border-radius: 8px;
    color: {c["section"]};
    padding: 9px;
}}

QProgressBar {{
    background: {c["log"]};
    border: 1px solid {c["panel_border"]};
    border-radius: 6px;
    height: 12px;
    text-align: center;
    color: transparent;
}}

QProgressBar::chunk {{
    background: #5aa2e3;
    border-radius: 5px;
}}

QSplitter::handle {{
    background: {c["bg"]};
    width: 8px;
}}

QScrollBar:vertical {{
    background: {c["table"]};
    width: 12px;
    margin: 2px;
}}

QScrollBar::handle:vertical {{
    background: {c["button_border"]};
    border-radius: 5px;
    min-height: 32px;
}}

QScrollBar::handle:vertical:hover {{
    background: {c["section"]};
}}

QScrollBar::add-line:vertical, QScrollBar::sub-line:vertical {{
    height: 0px;
}}
"""
