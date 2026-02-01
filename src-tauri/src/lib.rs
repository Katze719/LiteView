mod preview;

use preview::{FrameData, PreviewState};
use scap::capturer::{Capturer, Options};
use scap::frame::{Frame, FrameType};
use scap::{get_all_targets, has_permission, is_supported, request_permission, Target};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};
use tauri::{Emitter, Manager, State};

#[derive(Debug, Clone, Serialize)]
struct TargetDto {
    index: usize,
    id: u32,
    title: String,
    kind: String, // "display" | "window"
}

struct ScapState {
    stop_requested: Arc<AtomicBool>,
    /// Shared state with preview thread - just overwrite frame, no channel backlog.
    preview_state: Arc<Mutex<Option<Arc<PreviewState>>>>,
}

impl Default for ScapState {
    fn default() -> Self {
        Self {
            stop_requested: Arc::new(AtomicBool::new(false)),
            preview_state: Arc::new(Mutex::new(None)),
        }
    }
}

#[tauri::command]
fn get_scap_targets() -> Result<Vec<TargetDto>, String> {
    if !is_supported() {
        return Err("Screen capture is not supported on this system.".to_string());
    }
    if !has_permission() {
        if !request_permission() {
            return Err("Screen capture permission was denied.".to_string());
        }
    }
    let targets = get_all_targets();
    Ok(targets
        .into_iter()
        .enumerate()
        .map(|(index, t)| {
            let (id, title, kind) = match &t {
                Target::Display(d) => (d.id, d.title.clone(), "display"),
                Target::Window(w) => (w.id, w.title.clone(), "window"),
            };
            TargetDto {
                index,
                id,
                title,
                kind: kind.to_string(),
            }
        })
        .collect())
}

/// Convert scap frame directly to minifb format (0x00RRGGBB u32).
/// No downscaling - send full resolution, minifb will handle display scaling.
fn frame_to_buffer(frame: &Frame) -> Option<(u32, u32, Vec<u32>)> {
    let (width, height, buffer) = match frame {
        Frame::BGRA(f) => {
            let buf = f
                .data
                .chunks_exact(4)
                .map(|c| {
                    let r = c[2] as u32;
                    let g = c[1] as u32;
                    let b = c[0] as u32;
                    (r << 16) | (g << 8) | b
                })
                .collect::<Vec<u32>>();
            (f.width as u32, f.height as u32, buf)
        }
        Frame::BGR0(f) => {
            let buf = f
                .data
                .chunks_exact(4)
                .map(|c| {
                    let r = c[2] as u32;
                    let g = c[1] as u32;
                    let b = c[0] as u32;
                    (r << 16) | (g << 8) | b
                })
                .collect::<Vec<u32>>();
            (f.width as u32, f.height as u32, buf)
        }
        Frame::RGB(f) => {
            let buf = f
                .data
                .chunks_exact(3)
                .map(|c| {
                    let r = c[0] as u32;
                    let g = c[1] as u32;
                    let b = c[2] as u32;
                    (r << 16) | (g << 8) | b
                })
                .collect::<Vec<u32>>();
            (f.width as u32, f.height as u32, buf)
        }
        Frame::RGBx(f) => {
            let buf = f
                .data
                .chunks_exact(4)
                .map(|c| {
                    let r = c[0] as u32;
                    let g = c[1] as u32;
                    let b = c[2] as u32;
                    (r << 16) | (g << 8) | b
                })
                .collect::<Vec<u32>>();
            (f.width as u32, f.height as u32, buf)
        }
        Frame::XBGR(f) => {
            let buf = f
                .data
                .chunks_exact(4)
                .map(|c| {
                    let r = c[3] as u32;
                    let g = c[2] as u32;
                    let b = c[1] as u32;
                    (r << 16) | (g << 8) | b
                })
                .collect::<Vec<u32>>();
            (f.width as u32, f.height as u32, buf)
        }
        Frame::BGRx(f) => {
            let buf = f
                .data
                .chunks_exact(4)
                .map(|c| {
                    let r = c[2] as u32;
                    let g = c[1] as u32;
                    let b = c[0] as u32;
                    (r << 16) | (g << 8) | b
                })
                .collect::<Vec<u32>>();
            (f.width as u32, f.height as u32, buf)
        }
        _ => return None,
    };
    if width == 0 || buffer.len() != (height as usize).saturating_mul(width as usize) {
        return None;
    }
    Some((width, height, buffer))
}

#[tauri::command]
fn start_scap_capture(
    target_index: Option<usize>,
    app_handle: tauri::AppHandle,
    state: State<ScapState>,
) -> Result<(), String> {
    if !is_supported() {
        return Err("Screen capture is not supported.".to_string());
    }
    if !has_permission() {
        if !request_permission() {
            return Err("Permission denied.".to_string());
        }
    }
    state.stop_requested.store(false, Ordering::Relaxed);

    // On Linux, get_all_targets() returns empty; target is chosen via portal when capture starts.
    let target = target_index.and_then(|idx| {
        let targets = get_all_targets();
        targets.into_iter().nth(idx)
    });

    let options = Options {
        fps: 60,
        show_cursor: true,
        show_highlight: false,
        target,
        crop_area: None,
        output_type: FrameType::BGRAFrame,
        output_resolution: scap::capturer::Resolution::Captured,
        excluded_targets: None,
        ..Default::default()
    };

    // Stop any existing preview
    if let Some(old_state) = state.preview_state.lock().unwrap().take() {
        old_state.running.store(false, Ordering::Relaxed);
    }

    // Create shared state for preview
    let preview_state = Arc::new(PreviewState::default());
    state
        .preview_state
        .lock()
        .unwrap()
        .replace(preview_state.clone());

    // Spawn preview window thread
    let preview_state_for_window = preview_state.clone();
    thread::spawn(move || {
        preview::run_preview_window(preview_state_for_window);
    });

    let stop_requested_clone = state.stop_requested.clone();

    // Spawn capture thread
    thread::spawn(move || {
        let mut capturer = match Capturer::build(options) {
            Ok(c) => c,
            Err(e) => {
                let _ = app_handle.emit("scap-error", e.to_string());
                preview_state.running.store(false, Ordering::Relaxed);
                return;
            }
        };
        capturer.start_capture();

        while !stop_requested_clone.load(Ordering::Relaxed)
            && preview_state.running.load(Ordering::Relaxed)
        {
            // Wait until preview has consumed the previous frame
            while preview_state.frame.lock().unwrap().is_some() {
                if !preview_state.running.load(Ordering::Relaxed)
                    || stop_requested_clone.load(Ordering::Relaxed)
                {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_micros(500));
            }

            // Drain scap's internal buffer - get frames until we hit a "live" one.
            // A buffered frame returns instantly; a live frame blocks until captured.
            let mut latest_frame = None;
            loop {
                let start = std::time::Instant::now();
                let frame = match capturer.get_next_frame() {
                    Ok(f) => f,
                    Err(_) => break,
                };
                latest_frame = Some(frame);

                // If it took >5ms to get this frame, it's likely live (not buffered)
                if start.elapsed().as_millis() > 5 {
                    break;
                }
                // Otherwise it was buffered - loop to get the next one
            }

            if let Some(frame) = latest_frame {
                if let Some((width, height, buffer)) = frame_to_buffer(&frame) {
                    *preview_state.frame.lock().unwrap() = Some(FrameData {
                        width,
                        height,
                        buffer,
                    });
                }
            }
        }
        capturer.stop_capture();
        preview_state.running.store(false, Ordering::Relaxed);
    });

    Ok(())
}

#[tauri::command]
fn stop_scap_capture(state: State<ScapState>) -> Result<(), String> {
    state.stop_requested.store(true, Ordering::Relaxed);
    if let Some(preview_state) = state.preview_state.lock().unwrap().take() {
        preview_state.running.store(false, Ordering::Relaxed);
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(ScapState::default())
        .invoke_handler(tauri::generate_handler![
            get_scap_targets,
            start_scap_capture,
            stop_scap_capture,
        ])
        .setup(|app| {
            let start_capture_i = MenuItem::with_id(
                app,
                "start_capture",
                "Select screen / Start capture",
                true,
                None::<&str>,
            )?;
            let stop_capture_i =
                MenuItem::with_id(app, "stop_capture", "Stop capture", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "Show LiteView", true, None::<&str>)?;
            let settings_i = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[
                    &start_capture_i,
                    &stop_capture_i,
                    &show_i,
                    &settings_i,
                    &quit_i,
                ],
            )?;

            let mut builder = TrayIconBuilder::new();
            if let Some(icon) = app.default_window_icon().cloned() {
                builder = builder.icon(icon);
            }
            let _tray = builder
                .menu(&menu)
                .show_menu_on_left_click(true)
                .tooltip("LiteView")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "start_capture" => {
                        let _ = app.emit("capture-start", ());
                    }
                    "stop_capture" => {
                        let _ = app.emit("capture-stop", ());
                    }
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.unminimize();
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "settings" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.unminimize();
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
