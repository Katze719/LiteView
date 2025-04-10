import sys
import os
import subprocess
import tempfile  # Import tempfile for creating temporary files
from PyQt6 import QtWidgets, QtGui, QtCore

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

    with tempfile.NamedTemporaryFile(suffix=".png", delete=False) as tmp_file:  # Use tempfile instead of subprocess
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
    Captures the entire screen of the specified monitor.
    Automatically detects whether to use Wayland (grim) or X11/Windows.
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
            raise ValueError(f"Invalid monitor index: {monitor_index}. {len(screens)} monitors are available.")
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
        self.setWindowFlags(QtCore.Qt.WindowType.FramelessWindowHint | QtCore.Qt.WindowType.WindowStaysOnTopHint)

        # Get the screen's aspect ratio
        screen = QtWidgets.QApplication.instance().screens()[monitor_index]
        geom = screen.geometry()
        self.aspect_ratio = geom.width() / geom.height()

        # Set initial size and minimum size based on the aspect ratio
        self.setGeometry(100, 100, 800, int(800 / self.aspect_ratio))  # Default window size
        min_width = 50
        min_height = int(min_width / self.aspect_ratio)
        self.setMinimumSize(min_width, min_height)

        # Remove layout and make QLabel fill the entire window
        self.image_label = QtWidgets.QLabel(self)
        self.image_label.setAlignment(QtCore.Qt.AlignmentFlag.AlignCenter)
        self.image_label.setSizePolicy(QtWidgets.QSizePolicy.Policy.Expanding, QtWidgets.QSizePolicy.Policy.Expanding)
        self.image_label.setGeometry(self.rect())  # Ensure QLabel fills the window
        self.setContentsMargins(0, 0, 0, 0)  # Remove margins

        # Set up a timer to periodically update the screen
        self.timer = QtCore.QTimer(self)
        self.timer.timeout.connect(self.update_screen)
        self.timer.start(15)  # Update every 15ms (~66 FPS)

        self.current_pixmap = None  # Store the current pixmap for resizing
        self.drag_position = None  # Track the drag position

    def resizeEvent(self, event):
        """
        Handle window resize events to maintain the aspect ratio and resize the window dynamically.
        """
        new_width = event.size().width()
        new_height = event.size().height()

        if new_width / new_height != self.aspect_ratio:
            if new_width / self.aspect_ratio > new_height:
                # Adjust width to match the height
                new_width = int(new_height * self.aspect_ratio)
            else:
                # Adjust height to match the width
                new_height = int(new_width / self.aspect_ratio)

            self.resize(new_width, new_height)  # Resize the window to maintain aspect ratio

        self.image_label.setGeometry(self.rect())  # Ensure QLabel resizes with the window
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

            # Zeichne den roten Punkt für die Mausposition
            painter = QtGui.QPainter(screen_image)
            painter.setRenderHint(QtGui.QPainter.RenderHint.Antialiasing)
            painter.setPen(QtGui.QPen(QtGui.QColor('red'), 0))
            painter.setBrush(QtGui.QColor('red'))

            # Hole die globale Mausposition
            global_mouse_pos = QtGui.QCursor.pos()

            # Ermittle die Geometrie des aktuellen Bildschirms
            screen_geom = QtWidgets.QApplication.instance().screens()[self.monitor_index].geometry()

            # Umrechnung der globalen Mausposition in relative Bildschirmkoordinaten
            mouse_x = global_mouse_pos.x() - screen_geom.x()
            mouse_y = global_mouse_pos.y() - screen_geom.y()

            # Optional: Nur zeichnen, wenn sich die Maus auf diesem Monitor befindet
            if 0 <= mouse_x <= screen_geom.width() and 0 <= mouse_y <= screen_geom.height():
                radius = 5  # Radius des roten Punkts
                painter.drawEllipse(mouse_x - radius, mouse_y - radius, 2 * radius, 2 * radius)

            painter.end()

            pixmap = QtGui.QPixmap.fromImage(screen_image)
            if not pixmap.isNull():
                self.current_pixmap = pixmap  # Speichere das aktuelle Pixmap für das Resizing
                self.image_label.setPixmap(
                    pixmap.scaled(
                        self.image_label.size(),
                        QtCore.Qt.AspectRatioMode.KeepAspectRatio,
                        QtCore.Qt.TransformationMode.SmoothTransformation
                    )
                )
            else:
                print("Error: Pixmap is empty.")
        except Exception as e:
            print(f"Error capturing the screen: {e}")

    def mousePressEvent(self, event):
        """
        Handle mouse press events to initiate window dragging.
        """
        if event.button() == QtCore.Qt.MouseButton.LeftButton:
            self.drag_position = event.globalPosition().toPoint() - self.frameGeometry().topLeft()
            event.accept()

    def mouseMoveEvent(self, event):
        """
        Handle mouse move events to move the window.
        """
        if event.buttons() == QtCore.Qt.MouseButton.LeftButton and self.drag_position is not None:
            self.move(event.globalPosition().toPoint() - self.drag_position)
            event.accept()

    def mouseReleaseEvent(self, event):
        """
        Handle mouse release events to stop window dragging.
        """
        if event.button() == QtCore.Qt.MouseButton.LeftButton:
            self.drag_position = None
            event.accept()

class ScreenPreview(QtWidgets.QWidget):
    """
    Ein Widget, das Vorschaubilder aller verfügbaren Bildschirme anzeigt und
    dem Benutzer ermöglicht, einen Monitor auszuwählen.
    """
    def __init__(self, screens):
        super().__init__()
        self.setWindowTitle("Select a Screen")
        self.setWindowFlags(QtCore.Qt.WindowType.WindowStaysOnTopHint)
        self.layout = QtWidgets.QGridLayout(self)
        self.screens = screens

        for index, screen in enumerate(screens):
            geom = screen.geometry()
            preview_label = QtWidgets.QLabel(self)
            preview_label.setAlignment(QtCore.Qt.AlignmentFlag.AlignCenter)
            preview_label.setStyleSheet("border: 1px solid black; padding: 5px;")
            # Feste Größe für die Vorschau (Seitenverhältnis wird beibehalten)
            preview_label.setFixedSize(int(geom.width() / (len(screens) * 1.1)), int((geom.width() / (len(screens) * 1.1) * geom.height() / geom.width())))

            # Screenshot mittels capture_screen_monitor aufnehmen
            try:
                image = capture_screen_monitor(index)
                pixmap = QtGui.QPixmap.fromImage(image)
                if not pixmap.isNull():
                    preview_pixmap = pixmap.scaled(
                        preview_label.size(),
                        QtCore.Qt.AspectRatioMode.KeepAspectRatio,
                        QtCore.Qt.TransformationMode.SmoothTransformation
                    )
                    preview_label.setPixmap(preview_pixmap)
                else:
                    preview_label.setText(f"Monitor {index}\nBild nicht verfügbar")
            except Exception as e:
                preview_label.setText(f"Monitor {index}\nFehler: {e}")

            # Klick-Event zur Auswahl des entsprechenden Monitors
            preview_label.mousePressEvent = lambda event, idx=index: self.select_screen(idx)
            self.layout.addWidget(preview_label, index // 3, index % 3)

        self.selected_screen = None

    def select_screen(self, index):
        """
        Setzt den ausgewählten Bildschirm und schließt die Vorschau.
        """
        self.selected_screen = index
        self.close()

def main():
    app = QtWidgets.QApplication(sys.argv)
    screens = app.screens()
    if not screens:
        print("No screens found. Please check your configuration.")
        sys.exit(1)

    # Show screen previews
    preview = ScreenPreview(screens)
    preview.show()  # Use show() instead of exec_()

    # Wait for the user to select a screen
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
