from __future__ import annotations

from typing import Optional

from PySide6.QtCore import QRectF, Qt
from PySide6.QtGui import QColor, QFont, QImage, QPainter, QPen
from PySide6.QtWidgets import QSizePolicy, QWidget

from ..core.runtime import DeviceRuntime, OutputRuntime


class PreviewWidget(QWidget):
    def __init__(self, parent: QWidget | None = None) -> None:
        super().__init__(parent)
        self._runtime: Optional[DeviceRuntime] = None
        self.setSizePolicy(QSizePolicy.Expanding, QSizePolicy.Expanding)
        self.setMinimumSize(320, 240)

    def set_runtime(self, runtime: Optional[DeviceRuntime]) -> None:
        self._runtime = runtime
        self.update()

    def paintEvent(self, event) -> None:
        painter = QPainter(self)
        painter.setRenderHint(QPainter.Antialiasing, False)

        rect = self.rect()
        painter.fillRect(rect, QColor(16, 16, 18))

        runtime = self._runtime
        if runtime is None or not runtime.outputs:
            painter.setPen(QColor(140, 140, 140))
            painter.setFont(QFont("Segoe UI", 10))
            painter.drawText(rect, Qt.AlignCenter, "No device preview")
            return

        runtime.fill_output_buffers()

        label_font = QFont("Segoe UI", 9)
        painter.setFont(label_font)
        metrics = painter.fontMetrics()
        label_padding = 4
        max_label_w = 0
        max_label_h = 0
        for out in runtime.outputs:
            label = f"{out.name} ({out.output_type})"
            max_label_w = max(max_label_w, metrics.horizontalAdvance(label))
            max_label_h = max(max_label_h, metrics.height())

        gutter_top = max_label_h + label_padding * 2 + 8
        padding_left = 16
        padding_top = max(16, int(gutter_top))
        padding_right = 16
        padding_bottom = 16

        avail_w = max(1, rect.width() - padding_left - padding_right)
        avail_h = max(1, rect.height() - padding_top - padding_bottom)
        scale = min(avail_w / runtime.canvas_width, avail_h / runtime.canvas_height)
        if scale <= 0:
            return

        scaled_w = runtime.canvas_width * scale
        scaled_h = runtime.canvas_height * scale
        origin_x = rect.left() + padding_left + (avail_w - scaled_w) / 2
        origin_y = rect.top() + padding_top + (avail_h - scaled_h) / 2

        border_pen = QPen(QColor(60, 60, 70))
        border_pen.setWidth(1)

        for out in runtime.outputs:
            self._paint_output(painter, out, origin_x, origin_y, scale, border_pen)

    def _paint_output(
        self,
        painter: QPainter,
        out: OutputRuntime,
        origin_x: float,
        origin_y: float,
        scale: float,
        border_pen: QPen,
    ) -> None:
        x = origin_x + out.left * scale
        y = origin_y + out.top * scale
        w = out.virtual_width * scale
        h = out.virtual_height * scale

        try:
            image = QImage(out.render_buffer, out.virtual_width, out.virtual_height, out.virtual_width * 3, QImage.Format.Format_RGB888)
        except Exception:
            return

        painter.drawImage(QRectF(x, y, w, h), image)
        painter.setPen(border_pen)
        painter.drawRect(QRectF(x, y, w, h))

        label = f"{out.name} ({out.output_type})"
        metrics = painter.fontMetrics()
        text_w = metrics.horizontalAdvance(label)
        text_h = metrics.height()

        label_padding = 4
        label_w = text_w + label_padding * 2
        label_h = text_h + label_padding * 2
        label_x = x
        label_y = y - label_h - 6
        label_rect = QRectF(label_x, label_y, label_w, label_h)
        painter.fillRect(label_rect, QColor(0, 0, 0, 160))
        painter.setPen(QColor(230, 230, 230))
        painter.drawText(
            QRectF(
                label_rect.left() + label_padding,
                label_rect.top() + label_padding / 2,
                text_w + label_padding,
                text_h + label_padding,
            ),
            Qt.AlignLeft | Qt.AlignVCenter,
            label,
        )
