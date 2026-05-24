from __future__ import annotations

from PySide6.QtWidgets import (
    QDialog,
    QFrame,
    QHBoxLayout,
    QScrollArea,
    QVBoxLayout,
)
from qfluentwidgets import CheckBox as FluentCheckBox

from zapret_core.conflicts import ConflictProcess
from ui.fluent_widgets import BodyLabel, PrimaryButton, PushButton, panel_card


class ConflictDialog(QDialog):
    def __init__(self, processes: list[ConflictProcess], action_label: str, parent=None) -> None:
        super().__init__(parent)
        self.choice = "cancel"
        self.processes = processes
        self.checkboxes: list[tuple[FluentCheckBox, ConflictProcess]] = []
        self.setWindowTitle("Possible VPN conflict")
        self.setMinimumWidth(620)

        layout = QVBoxLayout(self)
        layout.setContentsMargins(18, 18, 18, 18)
        layout.setSpacing(12)

        title = BodyLabel("Possible VPN/proxy conflict")
        title.setObjectName("DialogTitle")
        layout.addWidget(title)

        description = BodyLabel(
            "These processes can intercept traffic and may break zapret work. "
            f"Close them before you {action_label}, or continue anyway."
        )
        description.setWordWrap(True)
        description.setObjectName("DialogText")
        layout.addWidget(description)

        process_panel = panel_card("Detected Conflicts")
        process_layout = process_panel.layout()
        for process in processes:
            checkbox = FluentCheckBox()
            checkbox.setText(process.display_name)
            checkbox.setChecked(True)
            process_layout.addWidget(checkbox)
            self.checkboxes.append((checkbox, process))
        process_layout.addStretch(1)

        scroll = QScrollArea()
        scroll.setWidgetResizable(True)
        scroll.setFrameShape(QFrame.NoFrame)
        scroll.setMinimumHeight(130)
        scroll.setMaximumHeight(220)
        scroll.setWidget(process_panel)
        layout.addWidget(scroll)

        buttons = QFrame()
        button_layout = QHBoxLayout(buttons)
        button_layout.setContentsMargins(0, 0, 0, 0)
        button_layout.setSpacing(10)
        button_layout.addStretch(1)

        kill_btn = PrimaryButton("Kill selected")
        continue_btn = PushButton("Continue anyway")
        cancel_btn = PushButton("Cancel")
        kill_btn.setMinimumWidth(160)
        continue_btn.setMinimumWidth(160)
        cancel_btn.setMinimumWidth(110)

        button_layout.addWidget(kill_btn)
        button_layout.addWidget(continue_btn)
        button_layout.addWidget(cancel_btn)
        layout.addWidget(buttons)

        kill_btn.clicked.connect(lambda: self._choose("kill"))
        continue_btn.clicked.connect(lambda: self._choose("continue"))
        cancel_btn.clicked.connect(lambda: self._choose("cancel"))

    def _choose(self, choice: str) -> None:
        self.choice = choice
        self.accept()

    def selected_processes(self) -> list[ConflictProcess]:
        return [process for checkbox, process in self.checkboxes if checkbox.isChecked()]
