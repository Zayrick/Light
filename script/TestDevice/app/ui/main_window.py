from __future__ import annotations

import json
import queue
import sys
from pathlib import Path
from typing import Optional

from PySide6.QtCore import Qt, QTimer
from PySide6.QtGui import QColor, QFont, QPalette
from PySide6.QtWidgets import (
    QApplication,
    QFileDialog,
    QFormLayout,
    QGroupBox,
    QHBoxLayout,
    QInputDialog,
    QLabel,
    QLineEdit,
    QListWidget,
    QListWidgetItem,
    QMainWindow,
    QMessageBox,
    QPushButton,
    QPlainTextEdit,
    QSpinBox,
    QSplitter,
    QStackedWidget,
    QVBoxLayout,
    QWidget,
)

from ..core.config import (
    DEFAULT_DEVICE_NAME,
    DEFAULT_PIXEL_SIZE,
    DEFAULT_UDP_PORT,
    SCHEMA_VERSION,
    default_device_config,
    device_config_from_dict,
    device_config_to_dict,
    load_device_config,
)
from ..core.runtime import DeviceRuntime
from ..core.protocol import PROTOCOL_VERSION
from ..presets import PresetStore
from ..services.virtual_device import VirtualDeviceServer
from .editors import (
    LinearEditor,
    LinearOutputModel,
    MatrixEditor,
    MatrixOutputModel,
    OutputModel,
    SingleEditor,
    SingleOutputModel,
)
from .preview import PreviewWidget


def dark_palette() -> QPalette:
    palette = QPalette()
    palette.setColor(QPalette.Window, QColor(20, 20, 20))
    palette.setColor(QPalette.WindowText, QColor(230, 230, 230))
    palette.setColor(QPalette.Base, QColor(16, 16, 16))
    palette.setColor(QPalette.AlternateBase, QColor(24, 24, 24))
    palette.setColor(QPalette.ToolTipBase, QColor(230, 230, 230))
    palette.setColor(QPalette.ToolTipText, QColor(20, 20, 20))
    palette.setColor(QPalette.Text, QColor(230, 230, 230))
    palette.setColor(QPalette.Button, QColor(30, 30, 30))
    palette.setColor(QPalette.ButtonText, QColor(230, 230, 230))
    palette.setColor(QPalette.BrightText, QColor(255, 80, 80))
    palette.setColor(QPalette.Highlight, QColor(0, 122, 204))
    palette.setColor(QPalette.HighlightedText, QColor(255, 255, 255))
    return palette


class MainWindow(QMainWindow):
    def __init__(self, preset_root: Path, initial_config: Optional[Path] = None) -> None:
        super().__init__()
        self.setWindowTitle("TestDevice Studio")
        self.resize(1400, 820)

        self._preset_store = PresetStore(preset_root)
        self._current_preset_path: Optional[Path] = None

        self._outputs: list[OutputModel] = []
        self._loading = False
        self._dirty = False
        self._pending_restart = False

        self._server: Optional[VirtualDeviceServer] = None
        self._preview_runtime: Optional[DeviceRuntime] = None
        self._log_queue: queue.SimpleQueue[str] = queue.SimpleQueue()

        self._build_ui()
        self._load_presets()

        if initial_config:
            try:
                config = load_device_config(initial_config)
                self._apply_device_config(config)
            except Exception as exc:
                QMessageBox.warning(self, "Config Load Failed", str(exc))
                self._apply_device_config(default_device_config())
        else:
            self._apply_device_config(default_device_config())

        self._refresh_timer = QTimer(self)
        self._refresh_timer.setInterval(33)
        self._refresh_timer.timeout.connect(self._on_refresh_timer)
        self._refresh_timer.start()

    def closeEvent(self, event) -> None:
        if self._dirty:
            result = QMessageBox.question(
                self,
                "Unsaved Changes",
                "Discard unsaved changes and exit?",
                QMessageBox.Yes | QMessageBox.No,
                QMessageBox.No,
            )
            if result != QMessageBox.Yes:
                event.ignore()
                return
        self._stop_server()
        event.accept()

    def _build_ui(self) -> None:
        central = QWidget()
        self.setCentralWidget(central)

        splitter = QSplitter(Qt.Horizontal)
        root_layout = QVBoxLayout(central)
        root_layout.setContentsMargins(8, 8, 8, 8)
        root_layout.addWidget(splitter)

        left_panel = QWidget()
        right_panel = QWidget()
        splitter.addWidget(left_panel)
        splitter.addWidget(right_panel)
        splitter.setStretchFactor(0, 0)
        splitter.setStretchFactor(1, 1)

        left_layout = QVBoxLayout(left_panel)
        left_layout.setContentsMargins(12, 12, 12, 12)
        left_layout.setSpacing(12)

        right_layout = QVBoxLayout(right_panel)
        right_layout.setContentsMargins(12, 12, 12, 12)
        right_layout.setSpacing(12)

        presets_box = QGroupBox("Presets")
        presets_layout = QVBoxLayout(presets_box)
        self.preset_list = QListWidget()
        self.preset_list.setMinimumHeight(120)
        presets_layout.addWidget(self.preset_list, 1)

        presets_btns = QHBoxLayout()
        self.preset_new_btn = QPushButton("New")
        self.preset_save_btn = QPushButton("Save")
        self.preset_save_as_btn = QPushButton("Save As")
        self.preset_delete_btn = QPushButton("Delete")
        presets_btns.addWidget(self.preset_new_btn)
        presets_btns.addWidget(self.preset_save_btn)
        presets_btns.addWidget(self.preset_save_as_btn)
        presets_btns.addWidget(self.preset_delete_btn)
        presets_layout.addLayout(presets_btns)

        device_box = QGroupBox("Device")
        device_form = QFormLayout(device_box)
        fixed_name = f"TestDevice V{PROTOCOL_VERSION}"
        self.device_name_edit = QLineEdit(fixed_name)
        self.device_name_edit.setReadOnly(True)
        self.udp_port_spin = QSpinBox()
        self.udp_port_spin.setRange(1, 65535)
        self.udp_port_spin.setValue(DEFAULT_UDP_PORT)
        self.pixel_size_spin = QSpinBox()
        self.pixel_size_spin.setRange(1, 64)
        self.pixel_size_spin.setValue(DEFAULT_PIXEL_SIZE)
        device_form.addRow("Name", self.device_name_edit)
        device_form.addRow("UDP Port", self.udp_port_spin)
        device_form.addRow("Pixel Size", self.pixel_size_spin)

        io_row = QHBoxLayout()
        self.import_btn = QPushButton("Import JSON")
        self.export_btn = QPushButton("Export JSON")
        io_row.addWidget(self.import_btn)
        io_row.addWidget(self.export_btn)
        device_form.addRow(io_row)

        outputs_box = QGroupBox("Outputs")
        outputs_layout = QVBoxLayout(outputs_box)
        self.outputs_list = QListWidget()
        self.outputs_list.setMinimumHeight(180)
        outputs_layout.addWidget(self.outputs_list, 1)

        output_btns = QHBoxLayout()
        self.add_matrix_btn = QPushButton("Add Matrix")
        self.add_linear_btn = QPushButton("Add Linear")
        self.add_single_btn = QPushButton("Add Single")
        output_btns.addWidget(self.add_matrix_btn)
        output_btns.addWidget(self.add_linear_btn)
        output_btns.addWidget(self.add_single_btn)
        outputs_layout.addLayout(output_btns)

        self.remove_output_btn = QPushButton("Remove Output")
        outputs_layout.addWidget(self.remove_output_btn)

        editor_box = QGroupBox("Output Editor")
        editor_layout = QVBoxLayout(editor_box)
        self.stack = QStackedWidget()
        self.matrix_editor = MatrixEditor()
        self.linear_editor = LinearEditor()
        self.single_editor = SingleEditor()
        self.stack.addWidget(self.matrix_editor)
        self.stack.addWidget(self.linear_editor)
        self.stack.addWidget(self.single_editor)
        editor_layout.addWidget(self.stack, 1)

        left_layout.addWidget(presets_box)
        left_layout.addWidget(device_box)
        left_layout.addWidget(outputs_box, 1)
        left_layout.addWidget(editor_box, 2)

        service_box = QGroupBox("Service")
        service_layout = QHBoxLayout(service_box)
        self.status_label = QLabel("Stopped")
        self.status_label.setFont(QFont("Segoe UI", 10, QFont.Bold))
        self.start_btn = QPushButton("Start")
        self.stop_btn = QPushButton("Stop")
        self.restart_btn = QPushButton("Restart")
        service_layout.addWidget(self.status_label)
        service_layout.addStretch(1)
        service_layout.addWidget(self.start_btn)
        service_layout.addWidget(self.stop_btn)
        service_layout.addWidget(self.restart_btn)

        preview_box = QGroupBox("Preview")
        preview_layout = QVBoxLayout(preview_box)
        self.preview_widget = PreviewWidget()
        preview_layout.addWidget(self.preview_widget, 1)

        log_box = QGroupBox("Log")
        log_layout = QVBoxLayout(log_box)
        self.log_view = QPlainTextEdit()
        self.log_view.setReadOnly(True)
        self.log_view.setMaximumBlockCount(300)
        log_layout.addWidget(self.log_view)

        right_layout.addWidget(service_box)
        right_layout.addWidget(preview_box, 1)
        right_layout.addWidget(log_box, 0)

        self.preset_list.currentItemChanged.connect(self._on_preset_selected)
        self.preset_new_btn.clicked.connect(self._new_preset)
        self.preset_save_btn.clicked.connect(self._save_preset)
        self.preset_save_as_btn.clicked.connect(self._save_preset_as)
        self.preset_delete_btn.clicked.connect(self._delete_preset)

        # Name is fixed for the test device; no config edits.
        self.udp_port_spin.valueChanged.connect(self._on_config_changed)
        self.pixel_size_spin.valueChanged.connect(self._on_config_changed)
        self.import_btn.clicked.connect(self._import_json)
        self.export_btn.clicked.connect(self._export_json)

        self.outputs_list.currentRowChanged.connect(self._on_select_output)
        self.add_matrix_btn.clicked.connect(self._add_matrix_output)
        self.add_linear_btn.clicked.connect(self._add_linear_output)
        self.add_single_btn.clicked.connect(self._add_single_output)
        self.remove_output_btn.clicked.connect(self._remove_selected_output)

        self.matrix_editor.changed.connect(self._on_output_changed)
        self.linear_editor.changed.connect(self._on_output_changed)
        self.single_editor.changed.connect(self._on_output_changed)

        self.start_btn.clicked.connect(self._start_server)
        self.stop_btn.clicked.connect(self._stop_server)
        self.restart_btn.clicked.connect(self._restart_server)

        self._update_status()

    def _load_presets(self) -> None:
        self.preset_list.blockSignals(True)
        self.preset_list.clear()
        for info in self._preset_store.list_presets():
            item = QListWidgetItem(info.name)
            item.setData(Qt.UserRole, info.path)
            self.preset_list.addItem(item)
            if self._current_preset_path and info.path == self._current_preset_path:
                self.preset_list.setCurrentItem(item)
        self.preset_list.blockSignals(False)

    def _apply_device_config(self, config) -> None:
        self._loading = True
        try:
            self.device_name_edit.setText(f"TestDevice V{PROTOCOL_VERSION}")
            self.udp_port_spin.setValue(config.udp_port)
            self.pixel_size_spin.setValue(config.pixel_size)
            self._outputs = self._config_to_models(config)
            self._refresh_outputs_list(select_first=True)
            if self._server and self._server.running:
                self._dirty = True
                self._pending_restart = True
            else:
                self._dirty = False
                self._pending_restart = False
            self._update_title()
            self._sync_preview_runtime()
            self._update_status()
        finally:
            self._loading = False

    def _config_to_models(self, config) -> list[OutputModel]:
        models: list[OutputModel] = []
        for out in config.outputs:
            if out.output_type == "Matrix" and out.matrix is not None:
                m = MatrixOutputModel(id=out.id, name=out.name, width=out.matrix.width, height=out.matrix.height)
                for idx, v in enumerate(out.matrix.map):
                    if v is not None:
                        x = idx % out.matrix.width
                        y = idx // out.matrix.width
                        m.cells.add((x, y))
                m.clip_cells()
                models.append(m)
                continue
            if out.output_type == "Linear":
                models.append(LinearOutputModel(id=out.id, name=out.name, length=out.leds_count))
                continue
            if out.output_type == "Single":
                models.append(SingleOutputModel(id=out.id, name=out.name))
                continue
        return models

    def _build_device_config(self):
        outputs = []
        ids: set[str] = set()
        for out in self._outputs:
            cfg = out.to_output_config()
            oid = str(cfg.get("id", "")).strip()
            if not oid:
                raise ValueError("Output id cannot be empty")
            if oid in ids:
                raise ValueError(f"Duplicate output id: {oid}")
            ids.add(oid)
            outputs.append(cfg)

        data = {
            "schema_version": SCHEMA_VERSION,
            # Kept in config for UI/presets, but runtime identity is fixed & defined by Python.
            "device_name": f"TestDevice V{PROTOCOL_VERSION}",
            "udp_port": int(self.udp_port_spin.value()),
            "pixel_size": int(self.pixel_size_spin.value()),
            "outputs": outputs,
        }
        return device_config_from_dict(data)

    def _refresh_outputs_list(self, select_first: bool = False) -> None:
        current_row = self.outputs_list.currentRow()
        self.outputs_list.blockSignals(True)
        self.outputs_list.clear()

        for out in self._outputs:
            if isinstance(out, MatrixOutputModel):
                leds = len(out.cells)
                label = f"{out.name}  [Matrix]  {out.width}x{out.height}  leds={leds}"
            elif isinstance(out, LinearOutputModel):
                label = f"{out.name}  [Linear]  len={out.length}"
            else:
                label = f"{out.name}  [Single]"
            item = QListWidgetItem(label)
            self.outputs_list.addItem(item)

        if select_first and self.outputs_list.count() > 0:
            self.outputs_list.setCurrentRow(0)
        elif 0 <= current_row < self.outputs_list.count():
            self.outputs_list.setCurrentRow(current_row)

        self.outputs_list.blockSignals(False)

    def _on_select_output(self, row: int) -> None:
        if row < 0 or row >= len(self._outputs):
            return
        out = self._outputs[row]
        if isinstance(out, MatrixOutputModel):
            self.stack.setCurrentWidget(self.matrix_editor)
            self.matrix_editor.set_model(out)
        elif isinstance(out, LinearOutputModel):
            self.stack.setCurrentWidget(self.linear_editor)
            self.linear_editor.set_model(out)
        else:
            self.stack.setCurrentWidget(self.single_editor)
            self.single_editor.set_model(out)

    def _on_output_changed(self) -> None:
        if self._loading:
            return
        self._refresh_outputs_list(select_first=False)
        self._mark_dirty()

    def _on_config_changed(self) -> None:
        if self._loading:
            return
        self._mark_dirty()

    def _mark_dirty(self) -> None:
        self._dirty = True
        if self._server and self._server.running:
            self._pending_restart = True
        else:
            self._sync_preview_runtime()
        self._update_title()
        self._update_status()

    def _update_title(self) -> None:
        suffix = "*" if self._dirty else ""
        self.setWindowTitle(f"TestDevice Studio{suffix}")

    def _update_status(self) -> None:
        if self._server and self._server.running:
            text = "Running"
            if self._pending_restart:
                text += " (restart required)"
        else:
            text = "Stopped"
        self.status_label.setText(text)
        self.start_btn.setEnabled(self._server is None or not self._server.running)
        self.stop_btn.setEnabled(self._server is not None and self._server.running)
        self.restart_btn.setEnabled(self._server is not None and self._server.running)

    def _sync_preview_runtime(self) -> None:
        if self._server and self._server.running:
            return
        try:
            config = self._build_device_config()
        except Exception:
            return
        self._preview_runtime = DeviceRuntime(config)
        self._preview_runtime.mark_dirty()
        self.preview_widget.set_runtime(self._preview_runtime)

    def _next_output_id(self, prefix: str) -> str:
        existing = {o.id for o in self._outputs}
        idx = 1
        while True:
            candidate = f"{prefix}-{idx}"
            if candidate not in existing:
                return candidate
            idx += 1

    def _add_matrix_output(self) -> None:
        new_id = self._next_output_id("matrix")
        m = MatrixOutputModel(id=new_id, name=f"Matrix {len(self._outputs) + 1}")
        m.cells = {(x, y) for y in range(m.height) for x in range(m.width)}
        self._outputs.append(m)
        self._refresh_outputs_list(select_first=False)
        self.outputs_list.setCurrentRow(self.outputs_list.count() - 1)
        self._mark_dirty()

    def _add_linear_output(self) -> None:
        new_id = self._next_output_id("strip")
        s = LinearOutputModel(id=new_id, name=f"Strip {len(self._outputs) + 1}", length=60)
        self._outputs.append(s)
        self._refresh_outputs_list(select_first=False)
        self.outputs_list.setCurrentRow(self.outputs_list.count() - 1)
        self._mark_dirty()

    def _add_single_output(self) -> None:
        new_id = self._next_output_id("single")
        s = SingleOutputModel(id=new_id, name=f"Single {len(self._outputs) + 1}")
        self._outputs.append(s)
        self._refresh_outputs_list(select_first=False)
        self.outputs_list.setCurrentRow(self.outputs_list.count() - 1)
        self._mark_dirty()

    def _remove_selected_output(self) -> None:
        row = self.outputs_list.currentRow()
        if row < 0 or row >= len(self._outputs):
            return
        if len(self._outputs) <= 1:
            QMessageBox.warning(self, "Not Allowed", "Device must have at least one output")
            return
        del self._outputs[row]
        self._refresh_outputs_list(select_first=False)
        self._mark_dirty()

    def _new_preset(self) -> None:
        if not self._confirm_discard():
            return
        self._current_preset_path = None
        self._apply_device_config(default_device_config())
        self._log("New preset created")

    def _save_preset(self) -> None:
        if self._current_preset_path is None:
            self._save_preset_as()
            return
        name = self._current_preset_path.stem
        self._save_preset_named(name)

    def _save_preset_as(self) -> None:
        name, ok = QInputDialog.getText(self, "Save Preset", "Preset name:")
        if not ok or not name.strip():
            return
        self._save_preset_named(name.strip())

    def _save_preset_named(self, name: str) -> None:
        try:
            config = self._build_device_config()
        except Exception as exc:
            QMessageBox.critical(self, "Invalid Config", str(exc))
            return
        data = device_config_to_dict(config)
        path = self._preset_store.save_preset(name, data)
        self._current_preset_path = path
        self._dirty = False
        self._load_presets()
        self._update_title()
        self._log(f"Preset saved: {path.name}")

    def _delete_preset(self) -> None:
        item = self.preset_list.currentItem()
        if not item:
            return
        path = item.data(Qt.UserRole)
        if not path:
            return
        result = QMessageBox.question(
            self,
            "Delete Preset",
            f"Delete preset '{Path(path).stem}'?",
            QMessageBox.Yes | QMessageBox.No,
            QMessageBox.No,
        )
        if result != QMessageBox.Yes:
            return
        self._preset_store.delete_preset(Path(path))
        if self._current_preset_path and Path(path) == self._current_preset_path:
            self._current_preset_path = None
        self._load_presets()
        self._log("Preset deleted")

    def _on_preset_selected(self, current: Optional[QListWidgetItem], previous: Optional[QListWidgetItem]) -> None:
        if current is None:
            return
        path = current.data(Qt.UserRole)
        if not path:
            return
        if not self._confirm_discard():
            return
        try:
            raw = self._preset_store.load_preset(Path(path))
            config = device_config_from_dict(raw)
            self._current_preset_path = Path(path)
            self._apply_device_config(config)
            self._log(f"Preset loaded: {Path(path).name}")
        except Exception as exc:
            QMessageBox.critical(self, "Load Failed", str(exc))

    def _import_json(self) -> None:
        path, _ = QFileDialog.getOpenFileName(self, "Import Config", str(Path.cwd()), "JSON Files (*.json)")
        if not path:
            return
        if not self._confirm_discard():
            return
        try:
            config = load_device_config(Path(path))
            self._current_preset_path = None
            self._apply_device_config(config)
            self._log(f"Config imported: {Path(path).name}")
        except Exception as exc:
            QMessageBox.critical(self, "Import Failed", str(exc))

    def _export_json(self) -> None:
        try:
            config = self._build_device_config()
        except Exception as exc:
            QMessageBox.critical(self, "Invalid Config", str(exc))
            return
        path, _ = QFileDialog.getSaveFileName(
            self,
            "Export Config",
            str(Path.cwd() / "device_config.json"),
            "JSON Files (*.json)",
        )
        if not path:
            return
        data = device_config_to_dict(config)
        Path(path).write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding="utf-8")
        self._log(f"Config exported: {Path(path).name}")

    def _confirm_discard(self) -> bool:
        if not self._dirty:
            return True
        result = QMessageBox.question(
            self,
            "Unsaved Changes",
            "Discard current changes?",
            QMessageBox.Yes | QMessageBox.No,
            QMessageBox.No,
        )
        return result == QMessageBox.Yes

    def _start_server(self) -> None:
        if self._server and self._server.running:
            return
        try:
            config = self._build_device_config()
        except Exception as exc:
            QMessageBox.critical(self, "Invalid Config", str(exc))
            return
        self._server = VirtualDeviceServer(config, on_log=self._log)
        try:
            self._server.start()
        except Exception as exc:
            QMessageBox.critical(self, "Start Failed", str(exc))
            self._server = None
            return
        self._pending_restart = False
        self.preview_widget.set_runtime(self._server.runtime)
        self._update_status()

    def _stop_server(self) -> None:
        if self._server:
            self._server.stop()
            self._server = None
        self._pending_restart = False
        self._sync_preview_runtime()
        self._update_status()

    def _restart_server(self) -> None:
        if not self._server or not self._server.running:
            return
        self._stop_server()
        self._start_server()

    def _active_runtime(self) -> Optional[DeviceRuntime]:
        if self._server and self._server.running:
            return self._server.runtime
        return self._preview_runtime

    def _on_refresh_timer(self) -> None:
        runtime = self._active_runtime()
        if runtime and runtime.consume_dirty():
            self.preview_widget.update()
        self._flush_logs()

    def _log(self, message: str) -> None:
        self._log_queue.put(message)

    def _flush_logs(self) -> None:
        while not self._log_queue.empty():
            try:
                msg = self._log_queue.get_nowait()
            except Exception:
                break
            self.log_view.appendPlainText(msg)


def run_app(preset_root: Path, initial_config: Optional[Path] = None) -> int:
    app = QApplication(sys.argv)
    app.setStyle("Fusion")
    app.setPalette(dark_palette())

    window = MainWindow(preset_root, initial_config=initial_config)
    window.show()
    return app.exec()
