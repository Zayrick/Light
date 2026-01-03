from __future__ import annotations

import json
import sys
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any, Optional

try:
    from PySide6.QtCore import Qt
    from PySide6.QtGui import QColor, QFont, QPalette
    from PySide6.QtWidgets import (
        QApplication,
        QComboBox,
        QFileDialog,
        QFormLayout,
        QFrame,
        QHBoxLayout,
        QLabel,
        QLineEdit,
        QListWidget,
        QListWidgetItem,
        QMainWindow,
        QMessageBox,
        QPushButton,
        QSpinBox,
        QSplitter,
        QStackedWidget,
        QTableWidget,
        QTableWidgetItem,
        QVBoxLayout,
        QWidget,
    )
except Exception as e:  # pragma: no cover
    raise SystemExit(
        "PySide6 未安装。请先在 script/TestDevice/.venv 中执行: pip install -r requirements-gui.txt\n"
        f"Import error: {e}"
    )


SCHEMA_VERSION = 1


def _dark_palette() -> QPalette:
    # Simple dark theme (Fusion) palette.
    p = QPalette()
    p.setColor(QPalette.Window, QColor(20, 20, 20))
    p.setColor(QPalette.WindowText, QColor(230, 230, 230))
    p.setColor(QPalette.Base, QColor(16, 16, 16))
    p.setColor(QPalette.AlternateBase, QColor(24, 24, 24))
    p.setColor(QPalette.ToolTipBase, QColor(230, 230, 230))
    p.setColor(QPalette.ToolTipText, QColor(20, 20, 20))
    p.setColor(QPalette.Text, QColor(230, 230, 230))
    p.setColor(QPalette.Button, QColor(30, 30, 30))
    p.setColor(QPalette.ButtonText, QColor(230, 230, 230))
    p.setColor(QPalette.BrightText, QColor(255, 80, 80))
    p.setColor(QPalette.Highlight, QColor(0, 122, 204))
    p.setColor(QPalette.HighlightedText, QColor(255, 255, 255))
    return p


def _load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def _dump_json(path: Path, data: Any) -> None:
    path.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")


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

    # Fallback.
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
    cells: set[tuple[int, int]] = field(default_factory=set)  # enabled cells: (x, y)

    def clip_cells(self) -> None:
        self.cells = {(x, y) for (x, y) in self.cells if 0 <= x < self.width and 0 <= y < self.height}

    def to_output_config(self) -> dict[str, Any]:
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

    def to_output_config(self) -> dict[str, Any]:
        return {
            "id": self.id,
            "name": self.name,
            "output_type": "Linear",
            "leds_count": int(self.length),
            "length": int(self.length),
        }


OutputModel = MatrixOutputModel | LinearOutputModel


class MatrixEditor(QWidget):
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

        self.table = QTableWidget()
        self.table.setFrameShape(QFrame.NoFrame)
        self.table.setEditTriggers(QTableWidget.NoEditTriggers)
        self.table.setSelectionMode(QTableWidget.SingleSelection)
        self.table.setSelectionBehavior(QTableWidget.SelectItems)
        self.table.setFocusPolicy(Qt.NoFocus)
        self.table.setShowGrid(True)
        self.table.horizontalHeader().setVisible(False)
        self.table.verticalHeader().setVisible(False)

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

        # Wiring
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
        f = QFont()
        f.setPointSize(8)
        f.setBold(True)
        self.table.setFont(f)

    def _sync_fields_to_model(self) -> None:
        if not self._model:
            return
        self._model.id = self.id_edit.text().strip() or self._model.id
        self._model.name = self.name_edit.text().strip() or self._model.name

    def _rebuild_table(self) -> None:
        if not self._model:
            return
        w, h = self._model.width, self._model.height
        self.table.setRowCount(h)
        self.table.setColumnCount(w)

        cell_size = 20
        for x in range(w):
            self.table.setColumnWidth(x, cell_size)
        for y in range(h):
            self.table.setRowHeight(y, cell_size)

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

    def _fill_all(self) -> None:
        if not self._model:
            return
        self._model.cells = {
            (x, y) for y in range(self._model.height) for x in range(self._model.width)
        }
        self._refresh()

    def _clear(self) -> None:
        if not self._model:
            return
        self._model.cells.clear()
        self._refresh()

    def _invert(self) -> None:
        if not self._model:
            return
        all_cells = {(x, y) for y in range(self._model.height) for x in range(self._model.width)}
        self._model.cells = all_cells.difference(self._model.cells)
        self._refresh()

    def _refresh(self) -> None:
        if not self._model:
            return

        self._sync_fields_to_model()
        self._model.order = str(self.order_combo.currentData())

        # Compute mapping so we can render indices in cells.
        out_cfg = self._model.to_output_config()
        m = out_cfg["matrix"]
        mapping: list[Optional[int]] = m["map"]
        w, h = int(m["width"]), int(m["height"])

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


class LinearEditor(QWidget):
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


class MainWindow(QMainWindow):
    def __init__(self) -> None:
        super().__init__()
        self.setWindowTitle("TestDevice JSON Config Generator")
        self.resize(1100, 720)

        self.device_name_edit = QLineEdit("TestMatrix")
        self.udp_port_spin = QSpinBox()
        self.udp_port_spin.setRange(1, 65535)
        self.udp_port_spin.setValue(9999)
        self.pixel_size_spin = QSpinBox()
        self.pixel_size_spin.setRange(1, 64)
        self.pixel_size_spin.setValue(6)

        self.outputs_list = QListWidget()
        self.outputs_list.setMinimumWidth(240)

        self.add_matrix_btn = QPushButton("Add Matrix")
        self.add_strip_btn = QPushButton("Add Strip")
        self.remove_btn = QPushButton("Remove")

        self.load_btn = QPushButton("Load JSON…")
        self.save_btn = QPushButton("Save JSON…")

        self.stack = QStackedWidget()
        self.matrix_editor = MatrixEditor()
        self.linear_editor = LinearEditor()
        self.stack.addWidget(self.matrix_editor)  # 0
        self.stack.addWidget(self.linear_editor)  # 1

        left = QWidget()
        left_layout = QVBoxLayout(left)
        left_layout.setContentsMargins(12, 12, 12, 12)
        left_layout.setSpacing(10)

        device_form = QFormLayout()
        device_form.addRow("Device Name", self.device_name_edit)
        device_form.addRow("UDP Port", self.udp_port_spin)
        device_form.addRow("Pixel Size", self.pixel_size_spin)
        left_layout.addLayout(device_form)

        left_layout.addWidget(QLabel("Outputs"))
        left_layout.addWidget(self.outputs_list, 1)

        btn_row = QHBoxLayout()
        btn_row.addWidget(self.add_matrix_btn)
        btn_row.addWidget(self.add_strip_btn)
        left_layout.addLayout(btn_row)
        left_layout.addWidget(self.remove_btn)

        io_row = QHBoxLayout()
        io_row.addWidget(self.load_btn)
        io_row.addWidget(self.save_btn)
        left_layout.addLayout(io_row)

        splitter = QSplitter()
        splitter.addWidget(left)
        splitter.addWidget(self.stack)
        splitter.setStretchFactor(1, 1)

        self.setCentralWidget(splitter)

        self._outputs: list[OutputModel] = []
        self._add_default_output()

        # Wiring
        self.outputs_list.currentRowChanged.connect(self._on_select_output)
        self.add_matrix_btn.clicked.connect(self._add_matrix_output)
        self.add_strip_btn.clicked.connect(self._add_linear_output)
        self.remove_btn.clicked.connect(self._remove_selected_output)
        self.save_btn.clicked.connect(self._save_json)
        self.load_btn.clicked.connect(self._load_json)

    def _add_default_output(self) -> None:
        m = MatrixOutputModel(id="matrix", name="LED Matrix")
        m.cells = {(x, y) for y in range(m.height) for x in range(m.width)}  # dense by default
        self._outputs.append(m)
        self._refresh_outputs_list(select_last=True)

    def _refresh_outputs_list(self, select_last: bool = False) -> None:
        self.outputs_list.clear()
        for out in self._outputs:
            if isinstance(out, MatrixOutputModel):
                leds = len(out.cells)
                label = f"{out.name}  [Matrix]  {out.width}x{out.height}  leds={leds}"
            else:
                label = f"{out.name}  [Linear]  len={out.length}"
            item = QListWidgetItem(label)
            item.setData(Qt.UserRole, out)
            self.outputs_list.addItem(item)

        if select_last and self.outputs_list.count() > 0:
            self.outputs_list.setCurrentRow(self.outputs_list.count() - 1)
        elif self.outputs_list.count() > 0 and self.outputs_list.currentRow() < 0:
            self.outputs_list.setCurrentRow(0)

    def _on_select_output(self, row: int) -> None:
        if row < 0 or row >= len(self._outputs):
            return
        out = self._outputs[row]
        if isinstance(out, MatrixOutputModel):
            self.stack.setCurrentIndex(0)
            self.matrix_editor.set_model(out)
        else:
            self.stack.setCurrentIndex(1)
            self.linear_editor.set_model(out)

    def _add_matrix_output(self) -> None:
        idx = len(self._outputs) + 1
        m = MatrixOutputModel(id=f"matrix-{idx}", name=f"Matrix {idx}")
        self._outputs.append(m)
        self._refresh_outputs_list(select_last=True)

    def _add_linear_output(self) -> None:
        idx = len(self._outputs) + 1
        s = LinearOutputModel(id=f"strip-{idx}", name=f"Strip {idx}", length=60)
        self._outputs.append(s)
        self._refresh_outputs_list(select_last=True)

    def _remove_selected_output(self) -> None:
        row = self.outputs_list.currentRow()
        if row < 0 or row >= len(self._outputs):
            return
        del self._outputs[row]
        self._refresh_outputs_list(select_last=False)

    def _config_to_json(self) -> dict[str, Any]:
        outputs = []
        ids: set[str] = set()
        for out in self._outputs:
            out_cfg = out.to_output_config()
            oid = str(out_cfg.get("id", "")).strip()
            if not oid:
                raise ValueError("Output id cannot be empty")
            if oid in ids:
                raise ValueError(f"Duplicate output id: {oid}")
            ids.add(oid)
            outputs.append(out_cfg)

        return {
            "schema_version": SCHEMA_VERSION,
            "device_name": self.device_name_edit.text().strip() or "TestMatrix",
            "udp_port": int(self.udp_port_spin.value()),
            "pixel_size": int(self.pixel_size_spin.value()),
            "outputs": outputs,
        }

    def _save_json(self) -> None:
        try:
            data = self._config_to_json()
        except Exception as e:
            QMessageBox.critical(self, "Invalid Config", str(e))
            return

        path, _ = QFileDialog.getSaveFileName(
            self,
            "Save Config JSON",
            str(Path.cwd() / "example_config.json"),
            "JSON Files (*.json)",
        )
        if not path:
            return
        try:
            _dump_json(Path(path), data)
        except Exception as e:
            QMessageBox.critical(self, "Save Failed", str(e))

    def _load_json(self) -> None:
        path, _ = QFileDialog.getOpenFileName(
            self,
            "Load Config JSON",
            str(Path.cwd()),
            "JSON Files (*.json)",
        )
        if not path:
            return
        try:
            raw = _load_json(Path(path))
            self._apply_loaded_config(raw)
        except Exception as e:
            QMessageBox.critical(self, "Load Failed", str(e))

    def _apply_loaded_config(self, raw: Any) -> None:
        if not isinstance(raw, dict):
            raise ValueError("Config root must be an object")
        self.device_name_edit.setText(str(raw.get("device_name", "TestMatrix")))
        self.udp_port_spin.setValue(int(raw.get("udp_port", 9999)))
        self.pixel_size_spin.setValue(int(raw.get("pixel_size", 6)))

        outs = raw.get("outputs", [])
        if not isinstance(outs, list) or not outs:
            raise ValueError("Config.outputs must be a non-empty list")

        parsed: list[OutputModel] = []
        ids: set[str] = set()
        for o in outs:
            if not isinstance(o, dict):
                raise ValueError("Each output must be an object")
            oid = str(o.get("id", "")).strip()
            if not oid:
                raise ValueError("Output id cannot be empty")
            if oid in ids:
                raise ValueError(f"Duplicate output id: {oid}")
            ids.add(oid)
            name = str(o.get("name", "")).strip() or oid
            typ = str(o.get("output_type", "")).strip()
            if typ.lower() == "matrix" or typ == "Matrix":
                matrix = o.get("matrix", {})
                if not isinstance(matrix, dict):
                    raise ValueError(f"Output '{oid}' matrix must be an object")
                w = int(matrix.get("width", 1))
                h = int(matrix.get("height", 1))
                m = MatrixOutputModel(id=oid, name=name, width=w, height=h)
                # Reconstruct enabled cells from map (null => disabled).
                raw_map = matrix.get("map", [])
                if isinstance(raw_map, list):
                    for idx, v in enumerate(raw_map):
                        if v is not None:
                            x = idx % w
                            y = idx // w
                            m.cells.add((x, y))
                m.order = "row-major"
                m.clip_cells()
                parsed.append(m)
                continue
            if typ.lower() == "linear" or typ == "Linear":
                length = int(o.get("length", o.get("leds_count", 60)))
                parsed.append(LinearOutputModel(id=oid, name=name, length=length))
                continue

            raise ValueError(f"Unsupported output_type: {typ}")

        self._outputs = parsed
        self._refresh_outputs_list(select_last=False)


def main() -> int:
    app = QApplication(sys.argv)
    app.setStyle("Fusion")
    app.setPalette(_dark_palette())

    w = MainWindow()
    w.show()
    return app.exec()


if __name__ == "__main__":
    raise SystemExit(main())
