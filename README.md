# LiteView

A small desktop app that shows another screen (or window) in a draggable,
always-on-top picture-in-picture window. **Click-through** (forwarding clicks to
the real screen) is optional and not implemented yet.

Built with **Tauri v2**, SvelteKit, and TypeScript.

> **Note:** Linux is currently not supported (screen capture fails in the Tauri
> window). Use Windows or macOS, or run the frontend in a browser on Linux.

## How it works

- **Capture**: Uses the system’s native screen/window picker via
  **getDisplayMedia()** (Screen Capture API). On Linux Wayland this goes through
  **xdg-desktop-portal** (PipeWire); on Windows/macOS it uses the OS picker.
- **Render**: The captured stream is drawn to a **Canvas** (2D). A WebGL
  pipeline can be added later for effects or click-through coordinate mapping.

No Rust screen-capture code: everything runs in the webview with standard web
APIs.

## Features

- **Picture-in-picture**: “Select screen/window” opens the system picker; the
  chosen display is shown in the small window.
- **Draggable overlay**: Drag by the title bar; window stays on top.
- **System tray**: Tray icon with menu: Select screen / Start capture (opens
  picker), Stop capture, Show LiteView, Settings, Quit. Left-click opens the
  menu.
- **Click-through** (optional, not yet): Could be added later by mapping canvas
  coordinates to the captured surface.

## Platform support

- **Windows**: Supported (native picker).
- **Linux**: **Not yet supported.** Screen capture in the Tauri window fails with
  NotAllowedError (Wry/WebKit does not handle the display-capture permission
  request). Use the app in a browser at `http://localhost:1420` on Linux for
  now.
- **macOS**: Supported (native picker; may need screen recording permission in
  System Settings).

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

- `src/` – SvelteKit frontend (getDisplayMedia, canvas render).
- `src-tauri/` – Tauri v2 shell (window config, no capture logic).

## License

MIT
