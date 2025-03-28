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
    Automatically detects whether to use Wayland (grim) or X11.
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
        geom = screen.geometry()
        image = screen.grabWindow(0, geom.x(), geom.y(), geom.width(), geom.height()).toImage()
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
            pixmap = QtGui.QPixmap.fromImage(screen_image)
            if not pixmap.isNull():
                self.current_pixmap = pixmap  # Store the current pixmap
                self.image_label.setPixmap(pixmap.scaled(self.image_label.size(), QtCore.Qt.AspectRatioMode.KeepAspectRatio, QtCore.Qt.TransformationMode.SmoothTransformation))
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
    A widget to display previews of all available screens and allow the user to select one.
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
            preview_label.setFixedSize(200, int(200 * geom.height() / geom.width()))  # Maintain aspect ratio
            preview_label.mousePressEvent = lambda event, idx=index: self.select_screen(idx)
            self.layout.addWidget(preview_label, index // 3, index % 3)  # Arrange in a grid

        self.selected_screen = None

    def select_screen(self, index):
        """
        Set the selected screen index and close the preview window.
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
