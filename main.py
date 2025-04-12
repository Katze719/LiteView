import sys
import os
import subprocess
import tempfile  # Für temporäre Dateien
import pathlib
from PyQt6 import QtWidgets, QtGui, QtCore


# Globales Cursor-Konfigurationsobjekt
class CursorSettings:
    def __init__(self):
        self.enabled = True
        self.style = "circle"  # Optionen: "circle", "cross"
        self.size = 5
        self.thickness = 0  # 0 = kosmetische Linie
        self.color = QtGui.QColor("red")


cursor_settings = CursorSettings()


def capture_screen_monitor_wayland(monitor_index):
    """
    Nimmt einen Screenshot des angegebenen Monitors unter Wayland mit grim auf.
    """
    app = QtWidgets.QApplication.instance()
    if app is None:
        app = QtWidgets.QApplication(sys.argv)
    screens = app.screens()
    if not screens:
        raise RuntimeError("No screens found.")
    if monitor_index < 0 or monitor_index >= len(screens):
        raise ValueError(
            f"Invalid monitor index: {monitor_index}. {len(screens)} monitors available."
        )
    screen = screens[monitor_index]
    with tempfile.NamedTemporaryFile(suffix=".png", delete=False) as tmp_file:
        filename = tmp_file.name
    try:
        cmd = ["grim", "-o", screen.name(), filename]
        subprocess.run(cmd, check=True)
        image = QtGui.QImage(filename)
        if image.isNull():
            raise RuntimeError("Error capturing the screen: The image is empty.")
        return image
    finally:
        if os.path.exists(filename):
            os.remove(filename)


def capture_screen_monitor(monitor_index):
    """
    Nimmt einen Screenshot des angegebenen Monitors auf.
    Erkennt automatisch, ob Wayland (grim) oder X11/Windows verwendet wird.
    """
    if os.getenv("WAYLAND_DISPLAY"):
        return capture_screen_monitor_wayland(monitor_index)
    else:
        app = QtWidgets.QApplication.instance()
        if app is None:
            app = QtWidgets.QApplication(sys.argv)
        screens = app.screens()
        if not screens:
            raise RuntimeError("No screens found.")
        if monitor_index < 0 or monitor_index >= len(screens):
            raise ValueError(
                f"Invalid monitor index: {monitor_index}. {len(screens)} monitors available."
            )
        screen = screens[monitor_index]
        image = screen.grabWindow(0).toImage()
        if image.isNull():
            raise RuntimeError("Error capturing the screen: The image is empty.")
        return image


class ScreenViewer(QtWidgets.QWidget):
    def __init__(self, monitor_index):
        super().__init__()
        self.monitor_index = monitor_index
        self.setWindowTitle("Screen Viewer")
        self.setWindowFlags(
            QtCore.Qt.WindowType.FramelessWindowHint
            | QtCore.Qt.WindowType.WindowStaysOnTopHint
            | QtCore.Qt.WindowType.Tool
        )

        # Bildschirmgeometrie und Seitenverhältnis
        screen = QtWidgets.QApplication.instance().screens()[monitor_index]
        geom = screen.geometry()
        self.aspect_ratio = geom.width() / geom.height()

        # Standardgröße festlegen
        self.setGeometry(100, 100, 800, int(800 / self.aspect_ratio))
        min_width = 50
        min_height = int(min_width / self.aspect_ratio)
        self.setMinimumSize(min_width, min_height)

        # QLabel füllt das gesamte Fenster
        self.image_label = QtWidgets.QLabel(self)
        self.image_label.setAlignment(QtCore.Qt.AlignmentFlag.AlignCenter)
        self.image_label.setSizePolicy(
            QtWidgets.QSizePolicy.Policy.Expanding,
            QtWidgets.QSizePolicy.Policy.Expanding,
        )
        self.image_label.setGeometry(self.rect())
        self.setContentsMargins(0, 0, 0, 0)

        # Timer zum Aktualisieren des Bildschirms
        self.timer = QtCore.QTimer(self)
        self.timer.timeout.connect(self.update_screen)
        self.timer.start(15)  # ca. 66 FPS

        self.current_pixmap = None  # Zum Zwischenspeichern des Pixmaps
        self.drag_position = None  # Für das Fensterziehen

    def resizeEvent(self, event):
        new_width = event.size().width()
        new_height = event.size().height()
        if new_width / new_height != self.aspect_ratio:
            if new_width / self.aspect_ratio > new_height:
                new_width = int(new_height * self.aspect_ratio)
            else:
                new_height = int(new_width / self.aspect_ratio)
            self.resize(new_width, new_height)
        self.image_label.setGeometry(self.rect())
        if self.current_pixmap:
            self.image_label.setPixmap(
                self.current_pixmap.scaled(
                    self.image_label.size(),
                    QtCore.Qt.AspectRatioMode.KeepAspectRatio,
                    QtCore.Qt.TransformationMode.SmoothTransformation,
                )
            )
        super().resizeEvent(event)

    def update_screen(self):
        try:
            screen_image = capture_screen_monitor(self.monitor_index)
            painter = QtGui.QPainter(screen_image)
            painter.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
            # Cursor zeichnen – nur, wenn er aktiviert ist
            if cursor_settings.enabled:
                pen = QtGui.QPen(cursor_settings.color, cursor_settings.thickness)
                painter.setPen(pen)
                painter.setBrush(cursor_settings.color)
                global_mouse_pos = QtGui.QCursor.pos()
                screen_geom = (
                    QtWidgets.QApplication.instance()
                    .screens()[self.monitor_index]
                    .geometry()
                )
                mouse_x = global_mouse_pos.x() - screen_geom.x()
                mouse_y = global_mouse_pos.y() - screen_geom.y()
                if (
                    0 <= mouse_x <= screen_geom.width()
                    and 0 <= mouse_y <= screen_geom.height()
                ):
                    if cursor_settings.style == "circle":
                        painter.drawEllipse(
                            mouse_x - cursor_settings.size,
                            mouse_y - cursor_settings.size,
                            2 * cursor_settings.size,
                            2 * cursor_settings.size,
                        )
                    elif cursor_settings.style == "cross":
                        painter.drawLine(
                            mouse_x - cursor_settings.size,
                            mouse_y,
                            mouse_x + cursor_settings.size,
                            mouse_y,
                        )
                        painter.drawLine(
                            mouse_x,
                            mouse_y - cursor_settings.size,
                            mouse_x,
                            mouse_y + cursor_settings.size,
                        )
            painter.end()
            pixmap = QtGui.QPixmap.fromImage(screen_image)
            if not pixmap.isNull():
                self.current_pixmap = pixmap
                self.image_label.setPixmap(
                    pixmap.scaled(
                        self.image_label.size(),
                        QtCore.Qt.AspectRatioMode.KeepAspectRatio,
                        QtCore.Qt.TransformationMode.SmoothTransformation,
                    )
                )
            else:
                print("Error: Pixmap is empty.")
        except Exception as e:
            print(f"Error capturing the screen: {e}")

    def mousePressEvent(self, event):
        if event.button() == QtCore.Qt.MouseButton.LeftButton:
            self.drag_position = (
                event.globalPosition().toPoint() - self.frameGeometry().topLeft()
            )
            event.accept()

    def mouseMoveEvent(self, event):
        if (
            event.buttons() == QtCore.Qt.MouseButton.LeftButton
            and self.drag_position is not None
        ):
            self.move(event.globalPosition().toPoint() - self.drag_position)
            event.accept()

    def mouseReleaseEvent(self, event):
        if event.button() == QtCore.Qt.MouseButton.LeftButton:
            self.drag_position = None
            event.accept()

    def closeEvent(self, event):
        # Beim Schließen wird nur das Fenster ausgeblendet, damit der Tray-Icon erhalten bleibt.
        event.ignore()
        self.hide()


class ScreenPreview(QtWidgets.QDialog):
    """
    Ein Dialog, der Vorschaubilder aller verfügbaren Bildschirme anzeigt und
    dem Benutzer die Auswahl eines Monitors ermöglicht.
    """

    def __init__(self, screens):
        super().__init__()
        self.setWindowTitle("Bildschirm auswählen")
        self.setWindowFlags(QtCore.Qt.WindowType.WindowStaysOnTopHint)
        self.layout = QtWidgets.QGridLayout(self)
        self.screens = screens
        self.selected_screen = None

        for index, screen in enumerate(screens):
            geom = screen.geometry()
            preview_label = QtWidgets.QLabel(self)
            preview_label.setAlignment(QtCore.Qt.AlignmentFlag.AlignCenter)
            preview_label.setStyleSheet("border: 1px solid black; padding: 5px;")
            preview_label.setFixedSize(
                int(geom.width() / (len(screens) * 1.1)),
                int(
                    (geom.width() / (len(screens) * 1.1)) * geom.height() / geom.width()
                ),
            )
            try:
                image = capture_screen_monitor(index)
                pixmap = QtGui.QPixmap.fromImage(image)
                if not pixmap.isNull():
                    preview_pixmap = pixmap.scaled(
                        preview_label.size(),
                        QtCore.Qt.AspectRatioMode.KeepAspectRatio,
                        QtCore.Qt.TransformationMode.SmoothTransformation,
                    )
                    preview_label.setPixmap(preview_pixmap)
                else:
                    preview_label.setText(f"Monitor {index}\nBild nicht verfügbar")
            except Exception as e:
                preview_label.setText(f"Monitor {index}\nFehler: {e}")
            # Beim Klick den Bildschirm auswählen
            preview_label.mousePressEvent = lambda event, idx=index: self.select_screen(
                idx
            )
            self.layout.addWidget(preview_label, index // 3, index % 3)

    def select_screen(self, index):
        self.selected_screen = index
        self.accept()


class CursorSettingsDialog(QtWidgets.QDialog):
    def __init__(self, cursor_settings, parent=None):
        super().__init__(parent)
        self.setWindowTitle("Cursor Einstellungen")
        self.setWindowFlags(
            self.windowFlags() | QtCore.Qt.WindowType.WindowStaysOnTopHint
        )
        self.cursor_settings = cursor_settings

        layout = QtWidgets.QFormLayout(self)

        # Checkbox: Cursor anzeigen
        self.enabled_checkbox = QtWidgets.QCheckBox("Cursor anzeigen")
        self.enabled_checkbox.setChecked(self.cursor_settings.enabled)
        layout.addRow("Anzeigen", self.enabled_checkbox)

        # ComboBox: Cursor-Stil (Kreis oder Kreuz)
        self.style_combo = QtWidgets.QComboBox()
        self.style_combo.addItems(["Kreis", "Kreuz"])
        if self.cursor_settings.style == "circle":
            self.style_combo.setCurrentIndex(0)
        else:
            self.style_combo.setCurrentIndex(1)
        layout.addRow("Stil", self.style_combo)

        # SpinBox: Größe
        self.size_spin = QtWidgets.QSpinBox()
        self.size_spin.setRange(1, 10000)
        self.size_spin.setValue(self.cursor_settings.size)
        layout.addRow("Größe", self.size_spin)

        # SpinBox: Dicke
        self.thickness_spin = QtWidgets.QSpinBox()
        self.thickness_spin.setRange(0, 20)
        self.thickness_spin.setValue(self.cursor_settings.thickness)
        layout.addRow("Dicke", self.thickness_spin)

        # Farbauswahl: Button + Anzeige
        self.color_button = QtWidgets.QPushButton("Farbe auswählen")
        self.color_button.clicked.connect(self.select_color)
        self.color_label = QtWidgets.QLabel()
        self.color_label.setAutoFillBackground(True)
        self.update_color_label()
        color_layout = QtWidgets.QHBoxLayout()
        color_layout.addWidget(self.color_button)
        color_layout.addWidget(self.color_label)
        layout.addRow("Farbe", color_layout)

        # Voreinstellungen (ComboBox)
        self.predefined_combo = QtWidgets.QComboBox()
        self.predefined_combo.addItem("Standard")
        self.predefined_combo.addItem("Deaktiviert")
        self.predefined_combo.addItem("Helles Kreuz")
        layout.addRow("Voreinstellungen", self.predefined_combo)

        # OK / Abbrechen Buttons
        button_box = QtWidgets.QDialogButtonBox(
            QtWidgets.QDialogButtonBox.StandardButton.Ok
            | QtWidgets.QDialogButtonBox.StandardButton.Cancel
        )
        button_box.accepted.connect(self.accept)
        button_box.rejected.connect(self.reject)
        layout.addRow(button_box)

    def select_color(self):
        color = QtWidgets.QColorDialog.getColor(
            initial=self.cursor_settings.color, parent=self, title="Wähle Cursor-Farbe"
        )
        if color.isValid():
            self.cursor_settings.color = color
            self.update_color_label()

    def update_color_label(self):
        palette = self.color_label.palette()
        palette.setColor(QtGui.QPalette.ColorRole.Window, self.cursor_settings.color)
        self.color_label.setPalette(palette)
        self.color_label.setText(self.cursor_settings.color.name())

    def accept(self):
        self.cursor_settings.enabled = self.enabled_checkbox.isChecked()
        if self.style_combo.currentText() == "Kreis":
            self.cursor_settings.style = "circle"
        else:
            self.cursor_settings.style = "cross"
        self.cursor_settings.size = self.size_spin.value()
        self.cursor_settings.thickness = self.thickness_spin.value()

        preset = self.predefined_combo.currentText()
        if preset == "Deaktiviert":
            self.cursor_settings.enabled = False
        elif preset == "Helles Kreuz":
            self.cursor_settings.enabled = True
            self.cursor_settings.style = "cross"
            self.cursor_settings.size = 10
            self.cursor_settings.thickness = 2
            self.cursor_settings.color = QtGui.QColor("yellow")
        elif preset == "Standard":
            self.cursor_settings = CursorSettings()
        super().accept()


class SystemTrayIcon(QtWidgets.QSystemTrayIcon):
    def __init__(self, icon, parent=None):
        super().__init__(icon, parent)
        self.menu = QtWidgets.QMenu(parent)

        # Option: Bildschirm auswählen
        screen_action = self.menu.addAction("Bildschirm auswählen")
        screen_action.triggered.connect(self.on_select_screen)

        # Option: Programm starten (Mirror einschalten)
        start_action = self.menu.addAction("Programm starten")
        start_action.triggered.connect(self.on_start_program)

        # Option: Mirror ausschalten (Viewer schließen)
        stop_action = self.menu.addAction("Mirror ausschalten")
        stop_action.triggered.connect(self.on_stop_program)

        # Option: Cursor Einstellungen
        cursor_action = self.menu.addAction("Cursor Einstellungen")
        cursor_action.triggered.connect(self.on_cursor_settings)

        # Option: Beenden
        exit_action = self.menu.addAction("Beenden")
        exit_action.triggered.connect(QtWidgets.QApplication.quit)

        self.setContextMenu(self.menu)
        self.selected_screen = None
        self.viewer = None
        
        # Quick Settings Menu für Linksklick
        self.quick_menu = None
        self.activated.connect(self.on_tray_activated)

    def on_tray_activated(self, reason):
        # Nur bei Linksklick das Quick-Menu anzeigen
        if reason == QtWidgets.QSystemTrayIcon.ActivationReason.Trigger:
            self.show_quick_settings()
    
    def show_quick_settings(self):
        # Quick Settings Menu erstellen
        self.quick_menu = QtWidgets.QMenu()
        
        # Screens als Buttons hinzufügen
        screens = QtWidgets.QApplication.instance().screens()
        for i, screen in enumerate(screens):
            action = self.quick_menu.addAction(f"Bildschirm {i}")
            action.triggered.connect(lambda checked, idx=i: self.quick_select_screen(idx))
        
        self.quick_menu.addSeparator()
        
        # Cursor ein/aus Toggle
        cursor_toggle = self.quick_menu.addAction("Cursor: " + ("Ein" if cursor_settings.enabled else "Aus"))
        cursor_toggle.triggered.connect(self.toggle_cursor)
        
        # Ein/Aus Buttons
        if self.viewer and self.viewer.isVisible():
            stop_action = self.quick_menu.addAction("Mirror ausschalten")
            stop_action.triggered.connect(self.on_stop_program)
        else:
            start_action = self.quick_menu.addAction("Mirror einschalten")
            start_action.triggered.connect(self.on_start_program)
        
        # Menu an der Position des Mauszeigers anzeigen
        self.quick_menu.popup(QtGui.QCursor.pos())
    
    def quick_select_screen(self, index):
        self.selected_screen = index
        # Mirror gleich starten/aktualisieren
        if self.viewer is not None:
            self.viewer.close()
            self.viewer.deleteLater()
            self.viewer = None
        self.viewer = ScreenViewer(self.selected_screen)
        self.viewer.show()
        self.viewer.activateWindow()
    
    def toggle_cursor(self):
        cursor_settings.enabled = not cursor_settings.enabled

    def on_select_screen(self):
        screens = QtWidgets.QApplication.instance().screens()
        dialog = ScreenPreview(screens)
        if dialog.exec() == QtWidgets.QDialog.DialogCode.Accepted:
            new_screen = dialog.selected_screen
            # Wenn nichts ausgewählt wurde, Standard Monitor 0 verwenden
            if new_screen is None:
                new_screen = 0
            # Falls ein anderer Bildschirm gewählt wurde, Viewer neu starten
            if self.selected_screen != new_screen:
                self.selected_screen = new_screen
                if self.viewer is not None:
                    self.viewer.close()
                    self.viewer.deleteLater()
                    self.viewer = None
                # Automatisch den Mirror mit dem neuen Monitor starten
                self.on_start_program()

    def on_start_program(self):
        # Falls kein Bildschirm ausgewählt wurde, wird Monitor 0 genutzt
        if self.selected_screen is None:
            self.selected_screen = 0
        if self.viewer is not None:
            if self.viewer.monitor_index != self.selected_screen:
                self.viewer.close()
                self.viewer.deleteLater()
                self.viewer = None
        if self.viewer is None:
            self.viewer = ScreenViewer(self.selected_screen)
        self.viewer.show()
        self.viewer.activateWindow()

    def on_stop_program(self):
        if self.viewer is not None:
            self.viewer.close()
            self.viewer.deleteLater()
            self.viewer = None

    def on_cursor_settings(self):
        dialog = CursorSettingsDialog(cursor_settings)
        dialog.exec()


def main():
    app = QtWidgets.QApplication(sys.argv)
    app.setQuitOnLastWindowClosed(False)  # Damit das Programm im Tray bleibt
    screens = app.screens()
    if not screens:
        print("No screens found. Please check your configuration.")
        sys.exit(1)

    icon_path = (pathlib.Path(__file__).parent / "icon.jpg").resolve().as_posix()

    icon = QtGui.QIcon(icon_path)
    if icon.isNull():
        print("Icon nicht gefunden! Programm wird beendet.")
        sys.exit(1)

    tray_icon = SystemTrayIcon(icon)
    tray_icon.setToolTip("LiteView")
    tray_icon.show()

    sys.exit(app.exec())


if __name__ == "__main__":
    main()
