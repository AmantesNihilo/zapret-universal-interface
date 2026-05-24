from __future__ import annotations

from collections import defaultdict

from PySide6.QtCore import Qt
from PySide6.QtWidgets import (
    QAbstractItemView,
    QDialog,
    QHBoxLayout,
    QScrollArea,
    QVBoxLayout,
    QWidget,
)
from qfluentwidgets import CheckBox as FluentCheckBox
from qfluentwidgets import MessageBox

from zapret_core.targets import Target
from ui.fluent_widgets import BodyLabel, PrimaryButton, PushButton, panel_card


RU_TEXT = {
    "Test Settings": "Настройки теста",
    "Choose targets for preset tests": "Выберите цели для проверки пресетов",
    "Choose services for preset tests": "Выберите сервисы для проверки пресетов",
    "All": "Все",
    "Core": "База",
    "Video": "Видео",
    "Social": "Соцсети",
    "Games": "Игры",
    "None": "Ничего",
    "Select group": "Выбрать группу",
    "Save": "Сохранить",
    "Cancel": "Отмена",
    "Select at least one target.": "Выберите хотя бы одну цель.",
}


class TestSettingsDialog(QDialog):
    def __init__(
        self,
        targets: list[Target],
        selected_names: set[str],
        parent=None,
        language: str = "en",
    ) -> None:
        super().__init__(parent)
        self.language = language
        self.setWindowTitle(self.tr_text("Test Settings"))
        self.resize(640, 620)
        self.targets = targets
        self.service_targets: dict[str, list[Target]] = defaultdict(list)
        self.service_categories: dict[str, str] = {}
        self.checkboxes: dict[str, FluentCheckBox] = {}

        active_names = selected_names or {target.name for target in targets}
        for target in targets:
            self.service_targets[target.service].append(target)
            self.service_categories[target.service] = target.category
        active_services = {
            target.service
            for target in targets
            if target.name in active_names
        }

        layout = QVBoxLayout(self)
        layout.setContentsMargins(14, 14, 14, 14)
        layout.setSpacing(10)

        title = BodyLabel(self.tr_text("Choose services for preset tests"))
        title.setStyleSheet("font-size: 14pt; font-weight: 650;")
        layout.addWidget(title)

        quick_row = QHBoxLayout()
        self.all_btn = PushButton(self.tr_text("All"))
        self.core_btn = PushButton(self.tr_text("Core"))
        self.video_btn = PushButton(self.tr_text("Video"))
        self.social_btn = PushButton(self.tr_text("Social"))
        self.games_btn = PushButton(self.tr_text("Games"))
        self.none_btn = PushButton(self.tr_text("None"))
        quick_row.addWidget(self.all_btn)
        quick_row.addWidget(self.core_btn)
        quick_row.addWidget(self.video_btn)
        quick_row.addWidget(self.social_btn)
        quick_row.addWidget(self.games_btn)
        quick_row.addWidget(self.none_btn)
        quick_row.addStretch(1)
        layout.addLayout(quick_row)

        scroll = QScrollArea()
        scroll.setWidgetResizable(True)
        content = QWidget()
        content_layout = QVBoxLayout(content)
        content_layout.setSpacing(10)

        grouped: dict[str, list[str]] = defaultdict(list)
        for service, category in self.service_categories.items():
            grouped[category].append(service)

        for category in sorted(grouped, key=str.casefold):
            group = panel_card(self.tr_text(category))
            group_layout = group.layout()
            group_check = FluentCheckBox(self.tr_text("Select group"))
            group_check.setTristate(False)
            group_layout.addWidget(group_check)

            child_boxes: list[FluentCheckBox] = []
            for service in sorted(grouped[category], key=str.casefold):
                checkbox = FluentCheckBox(service)
                checkbox.setChecked(service in active_services)
                checkbox.setProperty("service_name", service)
                group_layout.addWidget(checkbox)
                self.checkboxes[service] = checkbox
                child_boxes.append(checkbox)

            def update_group_state(_state=None, boxes=child_boxes, group_box=group_check):
                checked_count = sum(1 for box in boxes if box.isChecked())
                group_box.blockSignals(True)
                group_box.setCheckState(
                    Qt.Checked
                    if checked_count == len(boxes)
                    else Qt.Unchecked
                )
                group_box.blockSignals(False)

            def set_group_state(state, boxes=child_boxes):
                checked = int(state) == Qt.Checked.value
                for box in boxes:
                    box.setChecked(checked)

            group_check.stateChanged.connect(set_group_state)
            for checkbox in child_boxes:
                checkbox.stateChanged.connect(update_group_state)
            update_group_state()

            content_layout.addWidget(group)

        content_layout.addStretch(1)
        scroll.setWidget(content)
        layout.addWidget(scroll, 1)

        buttons = QHBoxLayout()
        buttons.addStretch(1)
        ok_btn = PrimaryButton(self.tr_text("Save"))
        cancel_btn = PushButton(self.tr_text("Cancel"))
        ok_btn.setMinimumWidth(120)
        cancel_btn.setMinimumWidth(120)
        buttons.addWidget(ok_btn)
        buttons.addWidget(cancel_btn)
        layout.addLayout(buttons)
        ok_btn.clicked.connect(self.accept)
        cancel_btn.clicked.connect(self.reject)

        self.all_btn.clicked.connect(lambda: self.set_checked_by_category(None, True))
        self.none_btn.clicked.connect(lambda: self.set_checked_by_category(None, False))
        self.core_btn.clicked.connect(lambda: self.only_category("Core"))
        self.video_btn.clicked.connect(lambda: self.only_category("Video"))
        self.social_btn.clicked.connect(lambda: self.only_category("Social"))
        self.games_btn.clicked.connect(lambda: self.only_category("Games"))
        self.configure_smooth_scrolling()

    def configure_smooth_scrolling(self) -> None:
        for view in self.findChildren(QAbstractItemView):
            view.setVerticalScrollMode(QAbstractItemView.ScrollPerPixel)
            view.setHorizontalScrollMode(QAbstractItemView.ScrollPerPixel)
            view.verticalScrollBar().setSingleStep(18)
            view.horizontalScrollBar().setSingleStep(18)
        for area in self.findChildren(QScrollArea):
            area.verticalScrollBar().setSingleStep(18)
            area.horizontalScrollBar().setSingleStep(18)

    def selected_target_names(self) -> set[str]:
        selected: set[str] = set()
        for service, checkbox in self.checkboxes.items():
            if checkbox.isChecked():
                selected.update(target.name for target in self.service_targets[service])
        return selected

    def tr_text(self, text: str) -> str:
        if self.language == "ru":
            return RU_TEXT.get(text, text)
        return text

    def accept(self) -> None:
        if not self.selected_target_names():
            MessageBox(self.tr_text("Test Settings"), self.tr_text("Select at least one target."), self).exec()
            return
        super().accept()

    def set_checked_by_category(self, category: str | None, checked: bool) -> None:
        for service, checkbox in self.checkboxes.items():
            service_category = self.service_categories.get(service, "")
            if category is None or service_category.casefold() == category.casefold():
                checkbox.setChecked(checked)

    def only_category(self, category: str) -> None:
        for service, checkbox in self.checkboxes.items():
            service_category = self.service_categories.get(service, "")
            checkbox.setChecked(service_category.casefold() == category.casefold())
