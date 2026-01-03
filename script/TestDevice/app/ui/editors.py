from __future__ import annotations

from dataclasses import dataclass, field
from typing import Optional

from PySide6.QtCore import Qt, Signal
from PySide6.QtGui import QColor, QFont, QFontMetrics
from PySide6.QtWidgets import (
    QComboBox,
    QFormLayout,
    QFrame,
    QHBoxLayout,
    QLabel,
    QLineEdit,
    QPushButton,
    QSpinBox,
    QTableWidget,
    QTableWidgetItem,
    QVBoxLayout,
    QWidget,
)


def _order_label(value: str) -> str:
    return {
        "row-major": "Row-major (→, then ↓)",
        "serpentine-rows": "Serpentine rows (↔)",
        "col-major": "Column-major (↓, then →)",
        "serpentine-cols": "Serpentine cols (↕)",
    }.get(value, value)


def _iter_cells(width: int, height: int, order: str):
    if order == "row-major":
        for y in range(height):
            for x in range(width):
                yield x, y
        return

    if order == "serpentine-rows":
        for y in range(height):
            xs = range(width) if (y % 2 == 0) else range(width - 1, -1, -1)
            for x in xs:
                yield x, y
        return

    if order == "col-major":
        for x in range(width):
            for y in range(height):
                yield x, y
        return

    if order == "serpentine-cols":
        for x in range(width):
            ys = range(height) if (x % 2 == 0) else range(height - 1, -1, -1)
            for y in ys:
                yield x, y
        return

    for y in range(height):
        for x in range(width):
            yield x, y


@dataclass
class MatrixOutputModel:
    id: str
    name: str
    width: int = 48
    height: int = 27
    order: str = "row-major"
    cells: set[tuple[int, int]] = field(default_factory=set)

    def clip_cells(self) -> None:
        self.cells = {(x, y) for (x, y) in self.cells if 0 <= x < self.width and 0 <= y < self.height}

    def to_output_config(self) -> dict[str, object]:
        w, h = self.width, self.height
        mapping: list[Optional[int]] = [None] * (w * h)
        led_idx = 0
        for x, y in _iter_cells(w, h, self.order):
            if (x, y) in self.cells:
                mapping[y * w + x] = led_idx
                led_idx += 1

        return {
            "id": self.id,
            "name": self.name,
            "output_type": "Matrix",
            "leds_count": led_idx,
            "matrix": {
                "width": w,
                "height": h,
                "map": mapping,
            },
        }


@dataclass
class LinearOutputModel:
    id: str
    name: str
    length: int = 60

    def to_output_config(self) -> dict[str, object]:
        return {
            "id": self.id,
            "name": self.name,
            "output_type": "Linear",
            "leds_count": int(self.length),
            "length": int(self.length),
        }


@dataclass
class SingleOutputModel:
    id: str
    name: str

    def to_output_config(self) -> dict[str, object]:
        return {
            "id": self.id,
            "name": self.name,
            "output_type": "Single",
            "leds_count": 1,
        }


OutputModel = MatrixOutputModel | LinearOutputModel | SingleOutputModel


class ZoomableTableWidget(QTableWidget):
    def __init__(self, on_zoom, parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self._on_zoom = on_zoom

    def wheelEvent(self, event) -> None:
        if event.modifiers() & Qt.ControlModifier:
            delta = event.angleDelta().y()
            if delta != 0:
                step = 2 if delta > 0 else -2
                self._on_zoom(step)
            event.accept()
            return
        super().wheelEvent(event)


class MatrixEditor(QWidget):
    changed = Signal()

    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self._model: Optional[MatrixOutputModel] = None

        self.id_edit = QLineEdit()
        self.name_edit = QLineEdit()

        self.width_spin = QSpinBox()
        self.width_spin.setRange(1, 512)
        self.height_spin = QSpinBox()
        self.height_spin.setRange(1, 512)

        self.order_combo = QComboBox()
        for v in ("row-major", "serpentine-rows", "col-major", "serpentine-cols"):
            self.order_combo.addItem(_order_label(v), userData=v)

        self._cell_size = 20
        self._current_max_digits = 1
        self.table = ZoomableTableWidget(self._on_zoom)
        self.table.setFrameShape(QFrame.NoFrame)
        self.table.setEditTriggers(QTableWidget.NoEditTriggers)
        self.table.setSelectionMode(QTableWidget.SingleSelection)
        self.table.setSelectionBehavior(QTableWidget.SelectItems)
        self.table.setFocusPolicy(Qt.NoFocus)
        self.table.setShowGrid(True)
        self.table.horizontalHeader().setVisible(False)
        self.table.verticalHeader().setVisible(False)
        self.table.setTextElideMode(Qt.ElideNone)

        self.fill_btn = QPushButton("Fill")
        self.clear_btn = QPushButton("Clear")
        self.invert_btn = QPushButton("Invert")

        form = QFormLayout()
        form.addRow("ID", self.id_edit)
        form.addRow("Name", self.name_edit)

        dim_row = QHBoxLayout()
        dim_row.addWidget(QLabel("W"))
        dim_row.addWidget(self.width_spin)
        dim_row.addSpacing(8)
        dim_row.addWidget(QLabel("H"))
        dim_row.addWidget(self.height_spin)
        dim_row.addStretch(1)
        form.addRow("Size", dim_row)
        form.addRow("Index Order", self.order_combo)

        tools = QHBoxLayout()
        tools.addWidget(self.fill_btn)
        tools.addWidget(self.clear_btn)
        tools.addWidget(self.invert_btn)
        tools.addStretch(1)

        root = QVBoxLayout(self)
        root.addLayout(form)
        root.addLayout(tools)
        root.addWidget(self.table, 1)

        self.table.cellClicked.connect(self._on_cell_clicked)
        self.width_spin.valueChanged.connect(self._on_dims_changed)
        self.height_spin.valueChanged.connect(self._on_dims_changed)
        self.order_combo.currentIndexChanged.connect(self._refresh)
        self.id_edit.textEdited.connect(self._sync_fields_to_model)
        self.name_edit.textEdited.connect(self._sync_fields_to_model)
        self.fill_btn.clicked.connect(self._fill_all)
        self.clear_btn.clicked.connect(self._clear)
        self.invert_btn.clicked.connect(self._invert)

        self._set_table_font()

    def set_model(self, model: MatrixOutputModel) -> None:
        self._model = model
        self.id_edit.setText(model.id)
        self.name_edit.setText(model.name)
        self.width_spin.setValue(model.width)
        self.height_spin.setValue(model.height)
        idx = self.order_combo.findData(model.order)
        if idx >= 0:
            self.order_combo.setCurrentIndex(idx)
        self._rebuild_table()
        self._refresh()

    def _set_table_font(self) -> None:
        self._update_table_font(max_digits=1)

    def _update_table_font(self, max_digits: int) -> None:
        cell_size = max(8, self._cell_size)
        max_digits = max(1, max_digits)
        target_w = max(6, cell_size - 6)
        target_h = max(6, cell_size - 6)

        font = QFont()
        font.setBold(True)
        text = "8" * max_digits

        size = min(target_h, 24)
        while size > 6:
            font.setPixelSize(size)
            metrics = QFontMetrics(font)
            if metrics.horizontalAdvance(text) <= target_w and metrics.height() <= target_h:
                break
            size -= 1

        if size <= 6:
            font.setPixelSize(6)

        self.table.setFont(font)

    def _apply_cell_size(self) -> None:
        if not self._model:
            return
        w, h = self._model.width, self._model.height
        for x in range(w):
            self.table.setColumnWidth(x, self._cell_size)
        for y in range(h):
            self.table.setRowHeight(y, self._cell_size)

    def _on_zoom(self, step: int) -> None:
        new_size = max(10, min(64, self._cell_size + step))
        if new_size == self._cell_size:
            return
        self._cell_size = new_size
        self._apply_cell_size()
        self._update_table_font(self._current_max_digits)
        self.table.viewport().update()

    def _sync_fields_to_model(self) -> None:
        if not self._model:
            return
        self._model.id = self.id_edit.text().strip() or self._model.id
        self._model.name = self.name_edit.text().strip() or self._model.name
        self.changed.emit()

    def _rebuild_table(self) -> None:
        if not self._model:
            return
        w, h = self._model.width, self._model.height
        self.table.setRowCount(h)
        self.table.setColumnCount(w)
        self._apply_cell_size()

        self.table.setUpdatesEnabled(False)
        try:
            for y in range(h):
                for x in range(w):
                    item = QTableWidgetItem("")
                    item.setTextAlignment(Qt.AlignCenter)
                    item.setFlags(Qt.ItemIsEnabled | Qt.ItemIsSelectable)
                    self.table.setItem(y, x, item)
        finally:
            self.table.setUpdatesEnabled(True)

    def _on_dims_changed(self) -> None:
        if not self._model:
            return
        self._model.width = int(self.width_spin.value())
        self._model.height = int(self.height_spin.value())
        self._model.clip_cells()
        self._rebuild_table()
        self._refresh()
        self.changed.emit()

    def _on_cell_clicked(self, row: int, col: int) -> None:
        if not self._model:
            return
        key = (col, row)
        if key in self._model.cells:
            self._model.cells.remove(key)
        else:
            self._model.cells.add(key)
        self.table.clearSelection()
        self._refresh()
        self.changed.emit()

    def _fill_all(self) -> None:
        if not self._model:
            return
        self._model.cells = {(x, y) for y in range(self._model.height) for x in range(self._model.width)}
        self._refresh()
        self.changed.emit()

    def _clear(self) -> None:
        if not self._model:
            return
        self._model.cells.clear()
        self._refresh()
        self.changed.emit()

    def _invert(self) -> None:
        if not self._model:
            return
        all_cells = {(x, y) for y in range(self._model.height) for x in range(self._model.width)}
        self._model.cells = all_cells.difference(self._model.cells)
        self._refresh()
        self.changed.emit()

    def _refresh(self) -> None:
        if not self._model:
            return

        self._sync_fields_to_model()
        self._model.order = str(self.order_combo.currentData())

        out_cfg = self._model.to_output_config()
        m = out_cfg["matrix"]
        mapping: list[Optional[int]] = m["map"]
        w, h = int(m["width"]), int(m["height"])
        self._current_max_digits = self._max_digits(mapping)
        self._update_table_font(self._current_max_digits)

        enabled_bg = QColor(0, 122, 204)
        disabled_bg = QColor(36, 36, 36)
        enabled_fg = QColor(255, 255, 255)
        disabled_fg = QColor(140, 140, 140)

        self.table.setUpdatesEnabled(False)
        try:
            for y in range(h):
                for x in range(w):
                    cell = y * w + x
                    item = self.table.item(y, x)
                    if item is None:
                        continue
                    v = mapping[cell]
                    if v is None:
                        item.setText("")
                        item.setBackground(disabled_bg)
                        item.setForeground(disabled_fg)
                    else:
                        item.setText(str(v))
                        item.setBackground(enabled_bg)
                        item.setForeground(enabled_fg)
        finally:
            self.table.setUpdatesEnabled(True)

    def _max_digits(self, mapping: list[Optional[int]]) -> int:
        max_idx = 0
        for v in mapping:
            if v is not None:
                max_idx = max(max_idx, int(v))
        return len(str(max_idx)) if max_idx > 0 else 1



class LinearEditor(QWidget):
    changed = Signal()

    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self._model: Optional[LinearOutputModel] = None

        self.id_edit = QLineEdit()
        self.name_edit = QLineEdit()
        self.length_spin = QSpinBox()
        self.length_spin.setRange(1, 4096)

        form = QFormLayout(self)
        form.addRow("ID", self.id_edit)
        form.addRow("Name", self.name_edit)
        form.addRow("Length", self.length_spin)

        self.id_edit.textEdited.connect(self._sync)
        self.name_edit.textEdited.connect(self._sync)
        self.length_spin.valueChanged.connect(self._sync)

    def set_model(self, model: LinearOutputModel) -> None:
        self._model = model
        self.id_edit.setText(model.id)
        self.name_edit.setText(model.name)
        self.length_spin.setValue(model.length)

    def _sync(self) -> None:
        if not self._model:
            return
        self._model.id = self.id_edit.text().strip() or self._model.id
        self._model.name = self.name_edit.text().strip() or self._model.name
        self._model.length = int(self.length_spin.value())
        self.changed.emit()


class SingleEditor(QWidget):
    changed = Signal()

    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self._model: Optional[SingleOutputModel] = None

        self.id_edit = QLineEdit()
        self.name_edit = QLineEdit()

        form = QFormLayout(self)
        form.addRow("ID", self.id_edit)
        form.addRow("Name", self.name_edit)

        self.id_edit.textEdited.connect(self._sync)
        self.name_edit.textEdited.connect(self._sync)

    def set_model(self, model: SingleOutputModel) -> None:
        self._model = model
        self.id_edit.setText(model.id)
        self.name_edit.setText(model.name)

    def _sync(self) -> None:
        if not self._model:
            return
        self._model.id = self.id_edit.text().strip() or self._model.id
        self._model.name = self.name_edit.text().strip() or self._model.name
        self.changed.emit()
