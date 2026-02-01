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

const CAPTURE_FPS: u32 = 60;

#[derive(Debug, Clone, Serialize)]
struct TargetDto {
    index: usize,
    id: u32,
    title: String,
    kind: String,
}

struct CaptureState {
    stop_requested: Arc<AtomicBool>,
    preview_state: Arc<Mutex<Option<Arc<PreviewState>>>>,
}

impl Default for CaptureState {
    fn default() -> Self {
        Self {
            stop_requested: Arc::new(AtomicBool::new(false)),
            preview_state: Arc::new(Mutex::new(None)),
        }
    }
}

#[tauri::command]
fn get_capture_targets() -> Result<Vec<TargetDto>, String> {
    if !is_supported() {
        return Err("Screen capture is not supported on this system.".to_string());
    }
    if !has_permission() && !request_permission() {
        return Err("Screen capture permission was denied.".to_string());
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
    let expected_len = (height as usize).saturating_mul(width as usize);
    if width == 0 || buffer.len() != expected_len {
        return None;
    }
    Some((width, height, buffer))
}

#[tauri::command]
fn start_capture(
    target_index: Option<usize>,
    app_handle: tauri::AppHandle,
    state: State<CaptureState>,
) -> Result<(), String> {
    if !is_supported() {
        return Err("Screen capture is not supported.".to_string());
    }
    if !has_permission() && !request_permission() {
        return Err("Permission denied.".to_string());
    }
    state.stop_requested.store(false, Ordering::Relaxed);

    let target = target_index.and_then(|idx| get_all_targets().into_iter().nth(idx));

    let options = Options {
        fps: CAPTURE_FPS,
        show_cursor: true,
        show_highlight: false,
        target,
        crop_area: None,
        output_type: FrameType::BGRAFrame,
        output_resolution: scap::capturer::Resolution::Captured,
        excluded_targets: None,
        ..Default::default()
    };

    if let Some(old_state) = state.preview_state.lock().unwrap().take() {
        old_state.running.store(false, Ordering::Relaxed);
        old_state.frame_available.notify_one();
    }

    let preview_state = Arc::new(PreviewState::default());
    state
        .preview_state
        .lock()
        .unwrap()
        .replace(preview_state.clone());

    let preview_state_for_window = preview_state.clone();
    thread::spawn(move || preview::run_preview_window(preview_state_for_window));

    let stop_requested_clone = state.stop_requested.clone();

    thread::spawn(move || {
        let mut capturer = match Capturer::build(options) {
            Ok(c) => c,
            Err(e) => {
                let _ = app_handle.emit("capture-error", e.to_string());
                preview_state.running.store(false, Ordering::Relaxed);
                return;
            }
        };
        capturer.start_capture();

        while !stop_requested_clone.load(Ordering::Relaxed)
            && preview_state.running.load(Ordering::Relaxed)
        {
            let frame = match capturer.get_next_frame() {
                Ok(f) => f,
                Err(_) => break,
            };

            if let Some((width, height, buffer)) = frame_to_buffer(&frame) {
                *preview_state.frame.lock().unwrap() = Some(FrameData {
                    width,
                    height,
                    buffer,
                });
                preview_state.frame_available.notify_one();
            }
        }

        capturer.stop_capture();
        preview_state.running.store(false, Ordering::Relaxed);
        preview_state.frame_available.notify_one();
    });

    Ok(())
}

#[tauri::command]
fn stop_capture(state: State<CaptureState>) -> Result<(), String> {
    state.stop_requested.store(true, Ordering::Relaxed);
    if let Some(preview_state) = state.preview_state.lock().unwrap().take() {
        preview_state.running.store(false, Ordering::Relaxed);
        preview_state.frame_available.notify_one();
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(CaptureState::default())
        .invoke_handler(tauri::generate_handler![
            get_capture_targets,
            start_capture,
            stop_capture,
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
                    "show" | "settings" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.unminimize();
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
