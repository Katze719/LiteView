import sys
import cv2
import numpy as np
import subprocess
import tempfile
import os
from PyQt5 import QtWidgets, QtGui, QtCore
import time

def capture_screen_area(x, y, w, h):
    """
    Erfasst den Bildschirmbereich.
    Unter Windows wird dafür ImageGrab von PIL genutzt, unter anderen Systemen (z.B. Wayland) wird grim verwendet.
    """
    if os.name == "nt":  # Windows
        try:
            from PIL import ImageGrab
        except ImportError:
            print("Das Modul Pillow (PIL) ist nicht installiert. Bitte führe 'pip install Pillow' aus.")
            sys.exit(1)
        bbox = (x, y, x + w, y + h)
        img = ImageGrab.grab(bbox=bbox)
        # Konvertiere PIL-Image zu OpenCV-Bild (BGR-Format)
        img = cv2.cvtColor(np.array(img), cv2.COLOR_RGB2BGR)
        return img
    else:
        with tempfile.NamedTemporaryFile(suffix=".png", delete=False) as tmp_file:
            filename = tmp_file.name
        cmd = ["grim", "-g", f"{x},{y} {w}x{h}", filename]
        subprocess.run(cmd, check=True)
        image = cv2.imread(filename)
        os.remove(filename)
        return image

class AreaSelector(QtWidgets.QWidget):
    selection_done = QtCore.pyqtSignal(QtCore.QRect)

    def __init__(self):
        super().__init__()
        self.setWindowTitle("Bereich auswählen")
        self.setWindowState(QtCore.Qt.WindowFullScreen)
        # Entferne WA_TranslucentBackground wegen Wayland-Problemen
        self.setStyleSheet("background-color: rgba(0, 0, 0, 77);")  # Simuliert Transparenz
        # Statt eines einheitlichen dunklen Hintergrunds simulieren wir Transparenz mit einem Screenshot
        screen_geom = QtWidgets.QApplication.primaryScreen().geometry()
        try:
            full_image = capture_screen_area(screen_geom.x(), screen_geom.y(), screen_geom.width(), screen_geom.height())
            self.bg_pixmap = self.cv2_to_qpixmap(full_image)
        except Exception as e:
            print("Fehler beim Laden des Desktop-Hintergrundes:", e)
            self.bg_pixmap = None
        self.start = QtCore.QPoint()
        self.end = QtCore.QPoint()
        self.setCursor(QtGui.QCursor(QtCore.Qt.CrossCursor))
        self.is_selecting = False

    def mousePressEvent(self, event):
        self.start = event.pos()
        self.end = event.pos()
        self.is_selecting = True
        self.update()

    def mouseMoveEvent(self, event):
        if self.is_selecting:
            self.end = event.pos()
            self.update()

    def mouseReleaseEvent(self, event):
        self.end = event.pos()
        self.is_selecting = False
        self.update()
        rect = QtCore.QRect(self.start, self.end).normalized()
        self.selection_done.emit(rect)
        self.close()

    def paintEvent(self, event):
        painter = QtGui.QPainter(self)
        # Zeichne den Desktop-Hintergrund, falls verfügbar
        if self.bg_pixmap:
            painter.drawPixmap(self.rect(), self.bg_pixmap)
        else:
            # Fallback: halbtransparenter schwarzer Hintergrund
            painter.fillRect(self.rect(), QtGui.QColor(0, 0, 0, 77))
        # Zeichne Auswahlrechteck, falls aktiv
        if self.is_selecting:
            painter.setPen(QtGui.QPen(QtCore.Qt.red, 2))
            rect = QtCore.QRect(self.start, self.end)
            painter.drawRect(rect.normalized())

    def cv2_to_qpixmap(self, img):
        # Konvertierung von OpenCV-Bild zu QPixmap
        height, width = img.shape[:2]
        if len(img.shape) == 3:
            img = cv2.cvtColor(img, cv2.COLOR_BGR2RGB)
            qimg = QtGui.QImage(img.data, width, height, 3 * width, QtGui.QImage.Format_RGB888)
        else:
            bytes_per_line = width
            qimg = QtGui.QImage(img.data, width, height, bytes_per_line, QtGui.QImage.Format_Grayscale8)
        return QtGui.QPixmap.fromImage(qimg)

class OverlayWindow(QtWidgets.QWidget):
    def __init__(self, capture_rect):
        super().__init__()
        self.capture_rect = capture_rect
        self.show_rectangles = False  # Standard: keine farbigen Rechtecke
        self.setWindowTitle("Edge Detection Overlay")
        self.setWindowFlags(QtCore.Qt.FramelessWindowHint | QtCore.Qt.WindowStaysOnTopHint | QtCore.Qt.Tool)
        self.setAttribute(QtCore.Qt.WA_TranslucentBackground)
        self.initUI()
        self.timer = QtCore.QTimer()
        self.timer.timeout.connect(self.update_image)
        self.timer.start(500)  # Aktualisierung alle 1000ms

    def initUI(self):
        layout = QtWidgets.QVBoxLayout()

        # Bildanzeige
        self.image_label = QtWidgets.QLabel("Edge Detection Output")
        self.image_label.setStyleSheet("background-color: rgba(0, 0, 0, 150);")
        layout.addWidget(self.image_label)

        # Slider für unteren Schwellenwert
        lower_label = QtWidgets.QLabel("Lower Threshold: 50")
        lower_label.setStyleSheet("background-color: rgba(0, 0, 0, 150); color: white; padding: 5px;")
        layout.addWidget(lower_label)

        self.lower_slider = QtWidgets.QSlider(QtCore.Qt.Horizontal)
        self.lower_slider.setMinimum(0)
        self.lower_slider.setMaximum(255)
        self.lower_slider.setValue(50)
        self.lower_slider.setTickPosition(QtWidgets.QSlider.TicksBelow)
        self.lower_slider.setTickInterval(5)
        self.lower_slider.valueChanged.connect(lambda val: lower_label.setText(f"Lower Threshold: {val}"))
        layout.addWidget(self.lower_slider)

        # Slider für oberen Schwellenwert
        upper_label = QtWidgets.QLabel("Upper Threshold: 150")
        upper_label.setStyleSheet("background-color: rgba(0, 0, 0, 150); color: white; padding: 5px;")
        layout.addWidget(upper_label)

        self.upper_slider = QtWidgets.QSlider(QtCore.Qt.Horizontal)
        self.upper_slider.setMinimum(0)
        self.upper_slider.setMaximum(255)
        self.upper_slider.setValue(150)
        self.upper_slider.setTickPosition(QtWidgets.QSlider.TicksBelow)
        self.upper_slider.setTickInterval(5)
        self.upper_slider.valueChanged.connect(lambda val: upper_label.setText(f"Upper Threshold: {val}"))
        layout.addWidget(self.upper_slider)

        # Slider für apertureSize
        aperture_label = QtWidgets.QLabel("Aperture Size: 3")
        aperture_label.setStyleSheet("background-color: rgba(0, 0, 0, 150); color: white; padding: 5px;")
        layout.addWidget(aperture_label)

        self.aperture_slider = QtWidgets.QSlider(QtCore.Qt.Horizontal)
        self.aperture_slider.setMinimum(0)  # 0 -> 3, 1 -> 5, 2 -> 7
        self.aperture_slider.setMaximum(2)
        self.aperture_slider.setValue(0)
        self.aperture_slider.setTickPosition(QtWidgets.QSlider.TicksBelow)
        self.aperture_slider.setTickInterval(1)
        self.aperture_slider.valueChanged.connect(lambda val: aperture_label.setText(f"Aperture Size: {3 + 2 * val}"))
        layout.addWidget(self.aperture_slider)

        # Checkbox zum Umschalten der Anzeige
        self.rectangles_checkbox = QtWidgets.QCheckBox("Farbige Rechtecke anzeigen")
        self.rectangles_checkbox.setChecked(False)
        self.rectangles_checkbox.stateChanged.connect(self.toggle_rectangles)
        layout.addWidget(self.rectangles_checkbox)

        self.setLayout(layout)
        self.resize(400, 400)
        self.move(100, 100)

    def toggle_rectangles(self, state):
        self.show_rectangles = (state == QtCore.Qt.Checked)

    def update_image(self):
        x = self.capture_rect.x()
        y = self.capture_rect.y()
        w = self.capture_rect.width()
        h = self.capture_rect.height()
        try:
            image = capture_screen_area(x, y, w, h)
            if image is None:
                raise ValueError("Kein Bild erhalten.")
        except Exception as e:
            print("Fehler beim Screenshot: ", e)
            return

        # Erzeuge Graustufenbild und invertiere es
        gray = cv2.cvtColor(image, cv2.COLOR_BGR2GRAY)
        gray_inverted = cv2.bitwise_not(gray)
        lower_val = self.lower_slider.value()
        upper_val = self.upper_slider.value()
        aperture_size = 3 + 2 * self.aperture_slider.value()
        edges = cv2.Canny(gray_inverted, lower_val, upper_val, apertureSize=aperture_size)

        if self.show_rectangles:
            # Für farbige Rechtecke: Wandle das Kantenbild in ein BGR-Bild um
            output = cv2.cvtColor(edges, cv2.COLOR_GRAY2BGR)
            contours, _ = cv2.findContours(edges.copy(), cv2.RETR_EXTERNAL, cv2.CHAIN_APPROX_SIMPLE)
            for cnt in contours:
                epsilon = 0.02 * cv2.arcLength(cnt, True)
                approx = cv2.approxPolyDP(cnt, epsilon, True)
                if len(approx) == 4:
                    cv2.drawContours(output, [approx], 0, (0, 255, 0), 2)
        else:
            # Nur das Schwarz-Weiß-Kantenbild anzeigen
            output = edges

        pixmap = self.cv2_to_qpixmap(output)
        self.image_label.setPixmap(pixmap)
        self.image_label.setScaledContents(True)

    def cv2_to_qpixmap(self, img):
        if len(img.shape) == 3:
            height, width = img.shape[:2]
            img_rgb = cv2.cvtColor(img, cv2.COLOR_BGR2RGB)
            qimg = QtGui.QImage(img_rgb.data, width, height, 3 * width, QtGui.QImage.Format_RGB888)
        else:
            height, width = img.shape
            qimg = QtGui.QImage(img.data, width, height, width, QtGui.QImage.Format_Grayscale8)
        return QtGui.QPixmap.fromImage(qimg.copy())

    def mousePressEvent(self, event):
        if event.button() == QtCore.Qt.LeftButton:
            self.drag_position = event.globalPos() - self.frameGeometry().topLeft()
            event.accept()

    def mouseMoveEvent(self, event):
        if event.buttons() == QtCore.Qt.LeftButton:
            self.move(event.globalPos() - self.drag_position)
            event.accept()


# Globale Variable, um das Overlay am Leben zu halten
overlay_window = None

def main():
    app = QtWidgets.QApplication(sys.argv)
    app.setQuitOnLastWindowClosed(False)  # Anwendung bleibt aktiv, auch wenn das Auswahlfenster schließt.
    selector = AreaSelector()
    selector.selection_done.connect(lambda rect: on_area_selected(rect))
    selector.show()
    sys.exit(app.exec_())

def on_area_selected(rect):
    global overlay_window
    overlay_window = OverlayWindow(rect)
    overlay_window.show()

if __name__ == '__main__':
    main()
