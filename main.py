import sys
import os
import subprocess
import tempfile  # Import tempfile for creating temporary files
from PyQt6 import QtWidgets, QtGui, QtCore
import mss
import numpy as np
import cv2

# Schalter: Bei True wird mss als Capture-Methode verwendet
USE_MSS = True

def capture_screen_monitor_wayland(monitor_index):
    """
    Captures the entire screen of the specified monitor using grim (Wayland).
    """
    app = QtWidgets.QApplication.instance()
    if app is None:
        app = QtWidgets.QApplication(sys.argv)
    screens = app.screens()
    if not screens:
        raise RuntimeError("No screens found.")
    if monitor_index < 0 or monitor_index >= len(screens):
        raise ValueError(f"Invalid monitor index: {monitor_index}. {len(screens)} monitors are available.")
    screen = screens[monitor_index]
    geom = screen.geometry()

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

def capture_screen_monitor_mss(monitor_index):
    """
    Captures the entire screen of the specified monitor using mss.
    This method should work on Windows and Linux (X11/Wayland if supported).
    """
    with mss.mss() as sct:
        monitors = sct.monitors
        # Hinweis: sct.monitors[0] enthält alle Monitore; einzelne Monitore beginnen ab Index 1
        if monitor_index + 1 >= len(monitors):
            raise ValueError(f"Invalid monitor index for mss capturing: {monitor_index}.")
        monitor = monitors[monitor_index + 1]
        screenshot = sct.grab(monitor)
        img = np.array(screenshot)
        # Konvertiere von BGRA zu RGBA (damit QImage die Farben korrekt darstellt)
        img = cv2.cvtColor(img, cv2.COLOR_BGRA2RGBA)
        height, width, channels = img.shape
        bytes_per_line = channels * width
        qimg = QtGui.QImage(img.data, width, height, bytes_per_line, QtGui.QImage.Format_RGBA8888)
        return qimg.copy()  # Kopie erstellen, um die zugrundeliegenden Daten nicht zu verlieren

def capture_screen_monitor(monitor_index):
    """
    Captures the entire screen of the specified monitor.
    Uses mss if USE_MSS flag is True, otherwise falls back to the existing method.
    """
    if USE_MSS:
        return capture_screen_monitor_mss(monitor_index)
    else:
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
                raise ValueError(f"Invalid monitor index: {monitor_index}. {len(screens)} monitors are available.")
            screen = screens[monitor_index]
            geom = screen.geometry()
            image = screen.grabWindow(0, geom.x(), geom.y(), geom.width(), geom.height()).toImage()
            if image.isNull():
                raise RuntimeError("Error capturing the screen: The image is empty.")

            # Overlay den Mauszeiger auf das Bild
            cursor = QtGui.QCursor()
            cursor_pos = cursor.pos() - geom.topLeft()
            painter = QtGui.QPainter(image)
            painter.drawPixmap(cursor_pos, cursor.pixmap())
            painter.end()

            return image

class ScreenViewer(QtWidgets.QWidget):
    def __init__(self, monitor_index):
        super().__init__()
        self.monitor_index = monitor_index
        self.setWindowTitle("Screen Viewer")
        self.setWindowFlags(QtCore.Qt.WindowType.FramelessWindowHint | QtCore.Qt.WindowType.WindowStaysOnTopHint)

        # Ermitteln des Seitenverhältnisses des Monitors
        screen = QtWidgets.QApplication.instance().screens()[monitor_index]
        geom = screen.geometry()
        self.aspect_ratio = geom.width() / geom.height()

        # Setze initiale Fenstergröße und Mindestgröße basierend auf dem Seitenverhältnis
        self.setGeometry(100, 100, 800, int(800 / self.aspect_ratio))
        min_width = 50
        min_height = int(min_width / self.aspect_ratio)
        self.setMinimumSize(min_width, min_height)

        # QLabel füllt das gesamte Fenster
        self.image_label = QtWidgets.QLabel(self)
        self.image_label.setAlignment(QtCore.Qt.AlignmentFlag.AlignCenter)
        self.image_label.setSizePolicy(QtWidgets.QSizePolicy.Policy.Expanding, QtWidgets.QSizePolicy.Policy.Expanding)
        self.image_label.setGeometry(self.rect())
        self.setContentsMargins(0, 0, 0, 0)

        # Timer zum periodischen Aktualisieren des Bildschirms
        self.timer = QtCore.QTimer(self)
        self.timer.timeout.connect(self.update_screen)
        self.timer.start(15)  # etwa 66 FPS

        self.current_pixmap = None
        self.drag_position = None

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
            self.image_label.setPixmap(self.current_pixmap.scaled(
                self.image_label.size(),
                QtCore.Qt.AspectRatioMode.KeepAspectRatio,
                QtCore.Qt.TransformationMode.SmoothTransformation
            ))
        super().resizeEvent(event)

    def update_screen(self):
        try:
            screen_image = capture_screen_monitor(self.monitor_index)
            pixmap = QtGui.QPixmap.fromImage(screen_image)
            if not pixmap.isNull():
                self.current_pixmap = pixmap
                self.image_label.setPixmap(pixmap.scaled(self.image_label.size(),
                                                         QtCore.Qt.AspectRatioMode.KeepAspectRatio,
                                                         QtCore.Qt.TransformationMode.SmoothTransformation))
            else:
                print("Error: Pixmap is empty.")
        except Exception as e:
            print(f"Error capturing the screen: {e}")

    def mousePressEvent(self, event):
        if event.button() == QtCore.Qt.MouseButton.LeftButton:
            self.drag_position = event.globalPosition().toPoint() - self.frameGeometry().topLeft()
            event.accept()

    def mouseMoveEvent(self, event):
        if event.buttons() == QtCore.Qt.MouseButton.LeftButton and self.drag_position is not None:
            self.move(event.globalPosition().toPoint() - self.drag_position)
            event.accept()

    def mouseReleaseEvent(self, event):
        if event.button() == QtCore.Qt.MouseButton.LeftButton:
            self.drag_position = None
            event.accept()

class ScreenPreview(QtWidgets.QWidget):
    """
    Zeigt eine Vorschau aller verfügbaren Monitore an und erlaubt die Auswahl.
    """
    def __init__(self, screens):
        super().__init__()
        self.setWindowTitle("Select a Screen")
        self.setWindowFlags(QtCore.Qt.WindowType.FramelessWindowHint | QtCore.Qt.WindowType.WindowStaysOnTopHint)
        self.layout = QtWidgets.QGridLayout(self)
        self.screens = screens

        for index, screen in enumerate(screens):
            geom = screen.geometry()
            preview_label = QtWidgets.QLabel(self)
            preview_label.setAlignment(QtCore.Qt.AlignmentFlag.AlignCenter)
            preview_label.setText(f"Monitor {index}")
            preview_label.setStyleSheet("border: 1px solid black; padding: 5px;")
            preview_label.setFixedSize(200, int(200 * geom.height() / geom.width()))
            preview_label.mousePressEvent = lambda event, idx=index: self.select_screen(idx)
            self.layout.addWidget(preview_label, index // 3, index % 3)

        self.selected_screen = None

    def select_screen(self, index):
        self.selected_screen = index
        self.close()

def main():
    app = QtWidgets.QApplication(sys.argv)
    screens = app.screens()
    if not screens:
        print("No screens found. Please check your configuration.")
        sys.exit(1)

    # Zeige eine Vorschau der Monitore an
    preview = ScreenPreview(screens)
    preview.show()

    # Warten, bis der Nutzer einen Monitor auswählt
    app.exec()

    if preview.selected_screen is None:
        print("No screen selected. Exiting.")
        sys.exit(1)

    monitor_index = preview.selected_screen

    viewer = ScreenViewer(monitor_index)
    viewer.show()
    sys.exit(app.exec())

if __name__ == "__main__":
    main()
