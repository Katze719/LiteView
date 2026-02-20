# LiteView

Desktop app: live preview of a screen or window in a small always-on-top window. Tauri 2, SvelteKit, Rust (scap for capture).

- **Tray only**: Start/stop capture and open settings from the system tray. No main window except the settings panel.
- **Preview**: Separate window shows the capture; FPS in the title bar.
- **Settings**: FPS, output resolution. Stored and applied on next capture start. Resolution scaling and FPS throttling run in-app on Linux (PipeWire often ignores those options).

## Platforms

- **Linux**: PipeWire/portal. Install `libpipewire-0.3-dev`, `libspa-0.2-dev`, and the usual Tauri/WebKit deps.
- **Windows**: scap with patched windows-capture (see Cargo.toml `[patch.crates-io]`).
- **macOS**: Supported by scap; not regularly tested here.

## Dev

```bash
pnpm install
pnpm tauri dev
```

Build: `pnpm tauri build`. Installers go to `src-tauri/target/release/bundle/`.

## Releases

Push a tag `v*` (e.g. `v1.0.0`). GitHub Actions build .deb, .rpm, .exe, .msi and attach them to a draft release. Version in the release is taken from the tag (tauri.conf.json, Cargo.toml, package.json are overwritten in CI from the tag).

## Layout

- `src/` – Svelte UI (settings, tray events).
- `src-tauri/` – Rust: capture (scap), preview window (winit + wgpu), tray.

MIT
