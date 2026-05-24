from __future__ import annotations

import math

from PySide6.QtCore import Property, QEasingCurve, QPoint, QRectF, QPropertyAnimation, QTimer, Qt
from PySide6.QtGui import QAction, QColor, QLinearGradient, QPainter, QPainterPath, QPalette
from PySide6.QtWidgets import QHBoxLayout, QTableWidgetItem, QVBoxLayout, QWidget
from qfluentwidgets import (
    BodyLabel as FluentBodyLabel,
    CardWidget,
    ComboBox as FluentComboBox,
    PrimaryPushButton as FluentPrimaryPushButton,
    ProgressBar as FluentProgressBar,
    PushButton as FluentPushButton,
    SearchLineEdit as FluentSearchLineEdit,
    StrongBodyLabel as FluentStrongBodyLabel,
    SubtitleLabel as FluentSubtitleLabel,
    TableWidget as FluentTableWidget,
    TextEdit as FluentTextEdit,
    TitleLabel as FluentTitleLabel,
    isDarkTheme,
)
from qfluentwidgets.components.widgets.menu import MenuAnimationType
from qfluentwidgets.components.widgets.combo_box import ComboBoxMenu


SORT_VALUE_ROLE = 0x0100 + 100


class PushButton(FluentPushButton):
    def __init__(self, text: str = "", parent: QWidget | None = None) -> None:
        super().__init__(parent)
        if text:
            self.setText(text)


class PrimaryButton(FluentPrimaryPushButton):
    def __init__(self, text: str = "", parent: QWidget | None = None) -> None:
        super().__init__(parent)
        if text:
            self.setText(text)


class SearchLineEdit(FluentSearchLineEdit):
    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)


class TextEdit(FluentTextEdit):
    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)


class ComboBox(FluentComboBox):
    def _createComboMenu(self) -> ComboBoxMenu:
        menu = ComboBoxMenu(self)
        menu.hBoxLayout.setContentsMargins(0, 0, 0, 0)
        menu.view.setViewportMargins(0, 0, 0, 0)
        menu.view.setVerticalScrollBarPolicy(Qt.ScrollBarAlwaysOff)
        menu.setStyleSheet("ComboBoxMenu, RoundMenu { background: transparent; border: none; }")
        if isDarkTheme():
            menu_bg = "#2b2b2b"
            item_hover = "#3a3a3a"
            text = "#f3f3f3"
        else:
            menu_bg = "#ffffff"
            item_hover = "#f2f2f2"
            text = "#1f1f1f"
        menu.view.setStyleSheet(
            f"""
            QListView#comboListWidget {{
                border: none;
                border-radius: 8px;
                background: {menu_bg};
                padding: 4px;
            }}
            QListView#comboListWidget::item {{
                min-height: 32px;
                padding: 0 12px;
                border-radius: 6px;
                color: {text};
                background: transparent;
            }}
            QListView#comboListWidget::item:selected,
            QListView#comboListWidget::item:hover {{
                background: {item_hover};
            }}
            """
        )
        menu.setShadowEffect(blurRadius=0, offset=(0, 0), color=QColor(0, 0, 0, 0))
        menu.view.setGraphicsEffect(None)
        menu.setWindowFlag(Qt.NoDropShadowWindowHint, True)
        return menu

    def _showComboMenu(self) -> None:
        if not self.items:
            return

        menu = self._createComboMenu()
        for item in self.items:
            action = QAction(item.icon, item.text)
            action.setEnabled(item.isEnabled)
            menu.addAction(action)

        menu.view.itemClicked.connect(lambda i: self._onItemClicked(self.findText(i.text().lstrip())))
        if menu.view.width() < self.width():
            menu.view.setMinimumWidth(self.width())
            menu.adjustSize()

        menu.setMaxVisibleItems(max(len(self.items), self.maxVisibleItems()))
        menu.setAttribute(Qt.WidgetAttribute.WA_DeleteOnClose)
        menu.closedSignal.connect(self._onDropMenuClosed)
        self.dropMenu = menu

        if self.currentIndex() >= 0 and self.items:
            menu.setDefaultAction(menu.actions()[self.currentIndex()])

        x = -menu.width() // 2 + menu.layout().contentsMargins().left() + self.width() // 2
        pos = self.mapToGlobal(QPoint(x, self.height()))
        menu.view.adjustSize(pos, MenuAnimationType.NONE)
        menu.exec(pos, aniType=MenuAnimationType.NONE)


class BodyLabel(FluentBodyLabel):
    def __init__(self, text: str = "", parent: QWidget | None = None) -> None:
        super().__init__(parent)
        if text:
            self.setText(text)


class StrongBodyLabel(FluentStrongBodyLabel):
    def __init__(self, text: str = "", parent: QWidget | None = None) -> None:
        super().__init__(parent)
        if text:
            self.setText(text)


class SubtitleLabel(FluentSubtitleLabel):
    def __init__(self, text: str = "", parent: QWidget | None = None) -> None:
        super().__init__(parent)
        if text:
            self.setText(text)


class TitleLabel(FluentTitleLabel):
    def __init__(self, text: str = "", parent: QWidget | None = None) -> None:
        super().__init__(parent)
        if text:
            self.setText(text)


class ProgressBar(FluentProgressBar):
    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)


class TableWidget(FluentTableWidget):
    def __init__(self, rows: int = 0, columns: int = 0, parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self.setRowCount(rows)
        self.setColumnCount(columns)


class SortableTableWidgetItem(QTableWidgetItem):
    def __lt__(self, other: QTableWidgetItem) -> bool:
        left = self.data(SORT_VALUE_ROLE)
        right = other.data(SORT_VALUE_ROLE)
        if left is not None and right is not None:
            return left < right
        return super().__lt__(other)


class FluentCard(CardWidget):
    def __init__(self, title: str = "", parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self.setObjectName("FluentCard")
        self.layout = QVBoxLayout(self)
        self.layout.setContentsMargins(16, 16, 16, 16)
        self.layout.setSpacing(10)
        if title:
            title_label = StrongBodyLabel(title)
            title_label.setObjectName("CardTitle")
            self.layout.addWidget(title_label)


class FluentCommandBar(CardWidget):
    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self.setObjectName("CommandBar")


class GlowServiceCard(QWidget):
    def __init__(self, accent: QColor, parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self.setObjectName("GlowServiceCard")
        self.setAttribute(Qt.WA_StyledBackground, False)
        self._accent = QColor(accent)
        self._active = False
        self._glow = 0.0
        self._phase = 0.0
        self._animation = QPropertyAnimation(self, b"glow", self)
        self._animation.setDuration(360)
        self._animation.setEasingCurve(QEasingCurve.OutCubic)
        self._animation.finished.connect(self._sync_motion_timer)
        self._motion_timer = QTimer(self)
        self._motion_timer.setInterval(33)
        self._motion_timer.timeout.connect(self._tick_motion)
        self.card_layout = QVBoxLayout(self)
        self.card_layout.setContentsMargins(20, 18, 20, 18)
        self.card_layout.setSpacing(12)

    def getGlow(self) -> float:
        return self._glow

    def setGlow(self, value: float) -> None:
        self._glow = max(0.0, min(1.0, float(value)))
        self.update()

    glow = Property(float, getGlow, setGlow)

    def set_running(self, running: bool, animate: bool = True) -> None:
        if self._active == running and self._animation.state() != QPropertyAnimation.Running:
            return
        self._active = running
        target = 1.0 if running else 0.0
        if not animate:
            self._animation.stop()
            self.setGlow(target)
            self._sync_motion_timer()
            return
        self._animation.stop()
        self._animation.setStartValue(self._glow)
        self._animation.setEndValue(target)
        self._animation.start()
        self._sync_motion_timer()

    def _sync_motion_timer(self) -> None:
        should_run = self._active or self._animation.state() == QPropertyAnimation.Running
        if should_run and not self._motion_timer.isActive():
            self._motion_timer.start()
        elif not should_run and self._motion_timer.isActive():
            self._motion_timer.stop()

    def _tick_motion(self) -> None:
        self._phase = (self._phase + 0.028) % (math.tau)
        if self._active or self._animation.state() == QPropertyAnimation.Running:
            self.update()
        else:
            self._motion_timer.stop()

    def paintEvent(self, event) -> None:
        if self.width() < 4 or self.height() < 4:
            super().paintEvent(event)
            return

        painter = QPainter(self)
        painter.setRenderHint(QPainter.Antialiasing, True)

        rect = QRectF(self.rect()).adjusted(1, 1, -1, -1)
        if rect.width() <= 0 or rect.height() <= 0:
            painter.end()
            super().paintEvent(event)
            return

        corner_radius = min(16.0, rect.width() / 2, rect.height() / 2)
        path = QPainterPath()
        path.addRoundedRect(rect, corner_radius, corner_radius)

        palette = self.palette()
        base = QColor(palette.color(QPalette.Window))
        if base.lightness() < 24:
            base = QColor(25, 28, 35)
        border = QColor(palette.color(QPalette.Mid))
        inactive = QColor(255, 82, 98) if base.lightness() < 128 else QColor(226, 68, 83)
        accent = QColor(self._accent)
        base_is_light = base.lightness() > 128
        mix = self._glow
        glow_color = QColor(
            int(inactive.red() * (1 - mix) + accent.red() * mix),
            int(inactive.green() * (1 - mix) + accent.green() * mix),
            int(inactive.blue() * (1 - mix) + accent.blue() * mix),
        )

        painter.fillPath(path, base)
        painter.save()
        painter.setClipPath(path)
        wave = 0.5 + 0.5 * math.sin(self._phase)
        drift = math.sin(self._phase * 0.72)
        lift = math.cos(self._phase * 0.58)
        start_x = rect.right() - rect.width() * (0.18 + 0.05 * drift)
        start_y = rect.top() + rect.height() * (0.10 + 0.08 * lift)
        end_x = rect.left() + rect.width() * 0.08
        end_y = rect.bottom() - rect.height() * 0.08
        active_alpha = int((36 if base_is_light else 62) + (16 if base_is_light else 24) * wave)
        mid_alpha = int((12 if base_is_light else 18) + (8 if base_is_light else 14) * wave)
        gradient = QLinearGradient(start_x, start_y, end_x, end_y)
        gradient.setColorAt(0.0, QColor(glow_color.red(), glow_color.green(), glow_color.blue(), active_alpha if self._active else 52))
        gradient.setColorAt(0.36, QColor(glow_color.red(), glow_color.green(), glow_color.blue(), mid_alpha if self._active else 18))
        gradient.setColorAt(1.0, QColor(glow_color.red(), glow_color.green(), glow_color.blue(), 0))
        painter.fillPath(path, gradient)

        if self._active:
            sweep = QLinearGradient(
                rect.left() + rect.width() * (0.12 + 0.08 * math.cos(self._phase * 0.9)),
                rect.bottom(),
                rect.left() + rect.width() * 0.72,
                rect.top(),
            )
            sweep.setColorAt(0.0, QColor(accent.red(), accent.green(), accent.blue(), int((14 if base_is_light else 20) + 10 * (1 - wave))))
            sweep.setColorAt(0.55, QColor(accent.red(), accent.green(), accent.blue(), 6 if base_is_light else 8))
            sweep.setColorAt(1.0, QColor(accent.red(), accent.green(), accent.blue(), 0))
            painter.fillPath(path, sweep)
        painter.restore()

        painter.setPen(border)
        painter.drawPath(path)
        painter.end()
        super().paintEvent(event)


def setting_row(title: str, control: QWidget) -> QWidget:
    row = QWidget()
    row.setObjectName("SettingRow")
    layout = QHBoxLayout(row)
    layout.setContentsMargins(0, 0, 0, 0)
    label = StrongBodyLabel(title)
    label.setObjectName("SectionTitle")
    layout.addWidget(label)
    layout.addStretch(1)
    layout.addWidget(control)
    return row


def panel_card(title: str = "") -> CardWidget:
    card = CardWidget()
    card.setObjectName("Panel")
    layout = QVBoxLayout(card)
    layout.setContentsMargins(16, 16, 16, 16)
    layout.setSpacing(10)
    if title:
        title_label = StrongBodyLabel(title)
        title_label.setObjectName("CardTitle")
        layout.addWidget(title_label)
    return card
