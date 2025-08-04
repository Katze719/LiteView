# 🌟 LiteView (Rust Edition)

A lightweight screen viewer application written in Rust with egui.

## ✨ Features

- 🖥️ **Multi-Monitor Support**: Preview all available monitors and select one to view
- 📏 **Dynamic Resizing**: Maintains aspect ratio while resizing the window
- ⚡ **Real-Time Updates**: Refreshes the screen content at ~30 FPS
- 🎨 **Modern UI**: Built with egui for a clean, modern interface
- 🛠️ **Cross-Platform**: Runs on Linux, Windows, and macOS
- 🖱️ **Cursor Overlay**: Optional cursor visualization with customizable styles

## 📦 Installation

### Prerequisites

- Rust 1.70 or higher
- Cargo (comes with Rust)

### Building from Source

1. Clone the repository:
   ```bash
   git clone https://github.com/Katze719/LiteView.git
   cd LiteView
   ```

2. Build the application:
   ```bash
   cargo build --release
   ```

3. Run the application:
   ```bash
   cargo run --release
   ```

## 🚀 Usage

1. Run the application:
   ```bash
   cargo run --release
   ```

2. A monitor selection window will appear showing all available displays
3. Click on the monitor you want to view
4. The selected monitor's content will be displayed in real-time
5. Use the settings panel to customize cursor display options

## 🛠️ Development

### Project Structure

```
src/
├── main.rs              # Main application entry point
├── screen_capture.rs    # Screen capture functionality
├── screen_viewer.rs     # Screen display and viewer
└── monitor_selector.rs  # Monitor selection interface
```

### Key Components

- **LiteViewApp**: Main application struct managing the overall state
- **ScreenCapture**: Handles screen capture using the `screenshots` crate
- **ScreenViewer**: Displays captured screen content with cursor overlay
- **MonitorSelector**: Provides monitor selection interface

### Dependencies

- `eframe`: egui framework for the GUI
- `egui`: Immediate mode GUI library
- `screenshots`: Cross-platform screen capture
- `image`: Image processing and manipulation
- `anyhow`: Error handling
- `tokio`: Async runtime
- `serde`: Serialization
- `tracing`: Logging

## 🎨 Customization

### Cursor Settings

- **Style**: Choose between Circle or Cross cursor styles
- **Size**: Adjust cursor size from 1-20 pixels
- **Color**: Customize cursor color using the color picker
- **Visibility**: Toggle cursor display on/off

## 🛡️ Requirements

- **Linux**: X11 or Wayland display server
- **Windows**: Windows 10 or later
- **macOS**: macOS 10.15 or later

## 🤝 Contributing

Contributions are welcome! Feel free to open issues or submit pull requests.

## 📄 License

This project is licensed under the GPL-3.0 License. See the [LICENSE](LICENSE) file for details.

## 🔧 Troubleshooting

### Common Issues

1. **Screen capture fails**: Ensure you have proper permissions for screen capture
2. **High CPU usage**: The application captures at ~30 FPS by default. You can modify the refresh rate in the code
3. **Display issues**: Try running with different display backends if available

### Performance Tips

- Use `--release` flag for optimal performance
- Close unnecessary applications to reduce system load
- Consider reducing capture frequency for better performance 
