# LiteView

A small desktop app that shows another screen (or window) in a draggable,
always-on-top picture-in-picture window. **Click-through** (forwarding clicks to
the real screen) is optional and not implemented yet.

Built with **Tauri v2**, SvelteKit, and TypeScript.

## How it works

- **Capture (Tauri)**: When running as the Tauri app, capture uses the
  **[scap](https://crates.io/crates/scap)** crate in Rust. The backend lists
  displays and windows, streams frames as PNG over Tauri events, and renders
  them on a canvas. This avoids the Linux **getDisplayMedia()** / WebKit
  permission issues and works on Windows, macOS, and Linux.
- **Capture (fallback)**: If scap targets are not available (e.g. in browser),
  the app falls back to **getDisplayMedia()** (Screen Capture API) with the
  system picker. On Linux Wayland this uses **xdg-desktop-portal** (PipeWire);
  on Windows/macOS the native picker.
- **Render**: The captured stream (or scap frames) is drawn to a **Canvas**
  (2D). A WebGL pipeline could be added later for effects or click-through
  coordinate mapping.

## Features

- **Picture-in-picture**: In Tauri, choose a screen or window from a dropdown
  and click “Start capture”. In browser mode, “Select screen/window” opens the
  system picker.
- **Draggable overlay**: Drag by the title bar; window stays on top.
- **System tray**: Tray icon with menu: Select screen / Start capture, Stop
  capture, Show LiteView, Settings, Quit. Left-click opens the menu.
- **Click-through** (optional, not yet): Could be added later by mapping canvas
  coordinates to the captured surface.

## Platform support

- **Windows**: Supported (scap in Tauri; getDisplayMedia in browser).
- **Linux**: Supported in the Tauri app via scap (no portal/WebKit display
  capture needed). In browser, getDisplayMedia may fail with NotAllowedError
  after picking a screen (WebKit/portal quirk); use the Tauri build for reliable
  capture.
- **macOS**: Supported (scap in Tauri; native picker in browser; may need screen
  recording permission in System Settings).

## Dev

### Prerequisites

- [Node.js](https://nodejs.org/) (LTS recommended)
- [pnpm](https://pnpm.io/) (`npm install -g pnpm`)
- [Rust](https://www.rust-lang.org/tools/install) (for Tauri)
- Platform-specific deps:
  [Tauri – Prerequisites](https://v2.tauri.app/start/prerequisites/)

### Run the app

```bash
pnpm install
pnpm tauri dev
```

Frontend is served at `http://localhost:1420`.

### Other commands

```bash
pnpm dev          # Frontend only
pnpm tauri build  # Production build
```

## Project structure

- **src/** – SvelteKit frontend: scap UI (target list, start/stop), canvas
  render, getDisplayMedia fallback, tray event listeners.
- **src-tauri/** – Tauri v2 shell: scap capture (get_scap_targets,
  start_scap_capture, stop_scap_capture), window config, system tray.

## Scap capture notes

- Frames are captured at 10 FPS and sent as base64 PNGs over the `scap-frame`
  event. Stop is requested via `stop_scap_capture`; the capture thread may take
  a moment to exit because `get_next_frame()` blocks until the next frame or
  source close.
- On Linux, scap uses the appropriate backend (e.g. PipeWire/portal or X11);
  ensure screen-sharing permission is granted when prompted.

## License

MIT
