from __future__ import annotations

from dataclasses import dataclass, field

from PySide6.QtCore import QAbstractTableModel, QModelIndex, QSortFilterProxyModel, Qt
from PySide6.QtGui import QBrush, QColor

from zapret_core import presets
from ui.fluent_widgets import SORT_VALUE_ROLE


@dataclass
class PresetTableRow:
    preset: presets.Preset
    favorite: bool = False
    checked: bool = True
    values: list[str] = field(default_factory=list)
    sort_values: list[int | str] = field(default_factory=list)

    def __post_init__(self) -> None:
        if not self.values:
            self.values = ["", "", self.preset.name, "Ready", "-", "-", "-", "-", "-", "-", "..."]
        if not self.sort_values:
            self.sort_values = [0, 1, self.preset.name.casefold(), "Ready", -1, -1, -1, -1, -1, -1, ""]

    @property
    def status(self) -> str:
        return self.values[3]

    @property
    def search_text(self) -> str:
        return f"{self.preset.name} {self.status} {self.preset.path}".casefold()


class PresetTableModel(QAbstractTableModel):
    def __init__(self, headers: list[str], parent=None) -> None:
        super().__init__(parent)
        self.headers = headers
        self.rows: list[PresetTableRow] = []
        self.theme = "dark"
        self.status_colors: dict[str, QColor | None] = {}

    def rowCount(self, parent: QModelIndex = QModelIndex()) -> int:
        return 0 if parent.isValid() else len(self.rows)

    def columnCount(self, parent: QModelIndex = QModelIndex()) -> int:
        return 0 if parent.isValid() else len(self.headers)

    def data(self, index: QModelIndex, role: int = Qt.DisplayRole):
        if not index.isValid():
            return None
        row = self.rows[index.row()]
        column = index.column()

        if role == Qt.DisplayRole:
            if column == 0:
                return "★" if row.favorite else "☆"
            if column == 1:
                return ""
            if column == 10:
                return "..."
            return row.values[column]
        if role == Qt.CheckStateRole and column == 1:
            return Qt.Checked if row.checked else Qt.Unchecked
        if role == Qt.TextAlignmentRole:
            if column in (0, 1, 4, 5, 6, 7, 8, 9, 10):
                return Qt.AlignCenter
            return Qt.AlignVCenter | Qt.AlignLeft
        if role == SORT_VALUE_ROLE:
            if column == 0:
                return 1 if row.favorite else 0
            if column == 1:
                return 1 if row.checked else 0
            return row.sort_values[column]
        if role == Qt.UserRole:
            return row.preset
        if role == Qt.BackgroundRole:
            return None
        if role == Qt.ForegroundRole and column == 0:
            return QBrush(QColor("#f6c85f") if row.favorite else self.muted_color())
        if role == Qt.ForegroundRole and column == 3:
            return QBrush(self.status_color(row.status))
        if role == Qt.ForegroundRole and column == 9:
            return QBrush(self.score_color(row.sort_values[column]))
        return None

    def headerData(self, section: int, orientation: Qt.Orientation, role: int = Qt.DisplayRole):
        if orientation == Qt.Horizontal and role == Qt.DisplayRole and 0 <= section < len(self.headers):
            return self.headers[section]
        return super().headerData(section, orientation, role)

    def flags(self, index: QModelIndex) -> Qt.ItemFlags:
        if not index.isValid():
            return Qt.NoItemFlags
        flags = Qt.ItemIsEnabled | Qt.ItemIsSelectable
        if index.column() == 1:
            flags |= Qt.ItemIsUserCheckable
        return flags

    def setData(self, index: QModelIndex, value, role: int = Qt.EditRole) -> bool:
        if not index.isValid():
            return False
        if index.column() == 1 and role == Qt.CheckStateRole:
            checked = value == Qt.Checked
            if self.rows[index.row()].checked == checked:
                return True
            self.rows[index.row()].checked = checked
            self.rows[index.row()].sort_values[1] = 1 if checked else 0
            self.dataChanged.emit(index, index, [Qt.CheckStateRole, SORT_VALUE_ROLE])
            return True
        return False

    def set_headers(self, headers: list[str]) -> None:
        self.headers = headers
        if headers:
            self.headerDataChanged.emit(Qt.Horizontal, 0, len(headers) - 1)

    def set_theme(self, theme: str, status_colors: dict[str, QColor | None]) -> None:
        self.theme = theme
        self.status_colors = status_colors
        if self.rows:
            top_left = self.index(0, 0)
            bottom_right = self.index(len(self.rows) - 1, len(self.headers) - 1)
            self.dataChanged.emit(top_left, bottom_right, [Qt.BackgroundRole, Qt.ForegroundRole])

    def set_rows(self, rows: list[PresetTableRow]) -> None:
        self.beginResetModel()
        self.rows = rows
        self.endResetModel()

    def row_for_preset(self, preset_name: str) -> int:
        for index, row in enumerate(self.rows):
            if row.preset.name == preset_name:
                return index
        return -1

    def preset_at(self, source_row: int) -> presets.Preset | None:
        if 0 <= source_row < len(self.rows):
            return self.rows[source_row].preset
        return None

    def set_favorite(self, source_row: int, favorite: bool) -> None:
        if not 0 <= source_row < len(self.rows):
            return
        self.rows[source_row].favorite = favorite
        self.rows[source_row].sort_values[0] = 1 if favorite else 0
        index = self.index(source_row, 0)
        self.dataChanged.emit(index, index, [Qt.DisplayRole, SORT_VALUE_ROLE])

    def set_all_checked(self, checked: bool) -> None:
        if not self.rows:
            return
        for row in self.rows:
            row.checked = checked
            row.sort_values[1] = 1 if checked else 0
        self.dataChanged.emit(self.index(0, 1), self.index(len(self.rows) - 1, 1), [Qt.CheckStateRole, SORT_VALUE_ROLE])

    def set_checked_for_favorites(self) -> None:
        if not self.rows:
            return
        for row in self.rows:
            row.checked = row.favorite
            row.sort_values[1] = 1 if row.checked else 0
        self.dataChanged.emit(self.index(0, 1), self.index(len(self.rows) - 1, 1), [Qt.CheckStateRole, SORT_VALUE_ROLE])

    def checked_presets(self) -> list[presets.Preset]:
        return [row.preset for row in self.rows if row.checked]

    def set_value(self, source_row: int, column: int, text: str, sort_value: int | str | None = None) -> None:
        if not 0 <= source_row < len(self.rows):
            return
        row = self.rows[source_row]
        row.values[column] = text
        if sort_value is not None:
            row.sort_values[column] = sort_value
        index = self.index(source_row, column)
        roles = [Qt.DisplayRole, SORT_VALUE_ROLE]
        if column in (3, 9):
            roles.extend([Qt.BackgroundRole, Qt.ForegroundRole])
        self.dataChanged.emit(index, index, roles)
        if column == 3:
            self.dataChanged.emit(self.index(source_row, 0), self.index(source_row, len(self.headers) - 1), [Qt.BackgroundRole])

    def row_status(self, source_row: int) -> str:
        if not 0 <= source_row < len(self.rows):
            return ""
        return self.rows[source_row].status

    def score_color(self, sort_value: int | str) -> QColor:
        try:
            score = int(sort_value)
        except (TypeError, ValueError):
            return QColor("#9aa4b5")
        if score >= 300:
            return QColor("#5ee089") if self.theme != "light" else QColor("#176b35")
        if score >= 150:
            return QColor("#ffd166") if self.theme != "light" else QColor("#8a6500")
        return QColor("#ff8ba0") if self.theme != "light" else QColor("#9b2638")

    def muted_color(self) -> QColor:
        return QColor("#9aa4b5") if self.theme != "light" else QColor("#6b7280")

    def status_color(self, status: str) -> QColor:
        if status in {"Passed", "Active", "Running"}:
            return QColor("#82e6a3") if self.theme != "light" else QColor("#176b35")
        if status in {"Partial", "Queued", "Working", "Stopping"}:
            return QColor("#ffd166") if self.theme != "light" else QColor("#8a6500")
        if status == "Failed":
            return QColor("#ff8ba0") if self.theme != "light" else QColor("#9b2638")
        if status == "Skipped":
            return self.muted_color()
        return QColor("#dce3ee") if self.theme != "light" else QColor("#1d2430")


class PresetFilterProxyModel(QSortFilterProxyModel):
    def __init__(self, parent=None) -> None:
        super().__init__(parent)
        self.filter_name = "all"
        self.search_text = ""
        self.setSortRole(SORT_VALUE_ROLE)
        self.setFilterCaseSensitivity(Qt.CaseInsensitive)

    def set_filter(self, filter_name: str, search_text: str) -> None:
        self.filter_name = filter_name
        self.search_text = search_text.strip().casefold()
        self.invalidateFilter()

    def filterAcceptsRow(self, source_row: int, source_parent: QModelIndex) -> bool:
        model = self.sourceModel()
        if not isinstance(model, PresetTableModel):
            return True
        if not 0 <= source_row < len(model.rows):
            return False
        row = model.rows[source_row]
        if self.filter_name == "passed" and row.status != "Passed":
            return False
        if self.filter_name == "working" and row.status not in {"Passed", "Partial"}:
            return False
        if self.filter_name == "favorites" and not row.favorite:
            return False
        if self.search_text and self.search_text not in row.search_text:
            return False
        return True
