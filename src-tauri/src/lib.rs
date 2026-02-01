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
const LIVE_FRAME_THRESHOLD_MS: u128 = 5;
const PREVIEW_WAIT_US: u64 = 500;

#[derive(Debug, Clone, Serialize)]
struct TargetDto {
    index: usize,
    id: u32,
    title: String,
    kind: String,
}

struct ScapState {
    stop_requested: Arc<AtomicBool>,
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

fn bgra_to_rgb888(c: &[u8]) -> u32 {
    let r = c[2] as u32;
    let g = c[1] as u32;
    let b = c[0] as u32;
    (r << 16) | (g << 8) | b
}

fn rgb_to_rgb888(c: &[u8]) -> u32 {
    let r = c[0] as u32;
    let g = c[1] as u32;
    let b = c[2] as u32;
    (r << 16) | (g << 8) | b
}

fn xbgr_to_rgb888(c: &[u8]) -> u32 {
    let r = c[3] as u32;
    let g = c[2] as u32;
    let b = c[1] as u32;
    (r << 16) | (g << 8) | b
}

fn frame_to_buffer(frame: &Frame) -> Option<(u32, u32, Vec<u32>)> {
    let (width, height, buffer) = match frame {
        Frame::BGRA(f) => (
            f.width as u32,
            f.height as u32,
            f.data.chunks_exact(4).map(bgra_to_rgb888).collect::<Vec<_>>(),
        ),
        Frame::BGR0(f) => (
            f.width as u32,
            f.height as u32,
            f.data.chunks_exact(4).map(bgra_to_rgb888).collect::<Vec<_>>(),
        ),
        Frame::RGB(f) => (
            f.width as u32,
            f.height as u32,
            f.data.chunks_exact(3).map(rgb_to_rgb888).collect::<Vec<_>>(),
        ),
        Frame::RGBx(f) => (
            f.width as u32,
            f.height as u32,
            f.data.chunks_exact(4).map(rgb_to_rgb888).collect::<Vec<_>>(),
        ),
        Frame::XBGR(f) => (
            f.width as u32,
            f.height as u32,
            f.data.chunks_exact(4).map(xbgr_to_rgb888).collect::<Vec<_>>(),
        ),
        Frame::BGRx(f) => (
            f.width as u32,
            f.height as u32,
            f.data.chunks_exact(4).map(bgra_to_rgb888).collect::<Vec<_>>(),
        ),
        _ => return None,
    };
    let expected_len = (height as usize).saturating_mul(width as usize);
    if width == 0 || buffer.len() != expected_len {
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
    if !has_permission() && !request_permission() {
        return Err("Permission denied.".to_string());
    }
    state.stop_requested.store(false, Ordering::Relaxed);

    let target = target_index.and_then(|idx| {
        get_all_targets().into_iter().nth(idx)
    });

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
    let wait_duration = std::time::Duration::from_micros(PREVIEW_WAIT_US);

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
            while preview_state.frame.lock().unwrap().is_some() {
                if !preview_state.running.load(Ordering::Relaxed)
                    || stop_requested_clone.load(Ordering::Relaxed)
                {
                    break;
                }
                std::thread::sleep(wait_duration);
            }

            let mut latest_frame = None;
            loop {
                let start = std::time::Instant::now();
                let frame = match capturer.get_next_frame() {
                    Ok(f) => f,
                    Err(_) => break,
                };
                latest_frame = Some(frame);
                if start.elapsed().as_millis() > LIVE_FRAME_THRESHOLD_MS {
                    break;
                }
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
            let start_capture_i =
                MenuItem::with_id(app, "start_capture", "Select screen / Start capture", true, None::<&str>)?;
            let stop_capture_i =
                MenuItem::with_id(app, "stop_capture", "Stop capture", true, None::<&str>)?;
            let show_i = MenuItem::with_id(app, "show", "Show LiteView", true, None::<&str>)?;
            let settings_i = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[&start_capture_i, &stop_capture_i, &show_i, &settings_i, &quit_i],
            )?;

            let mut builder = TrayIconBuilder::new();
            if let Some(icon) = app.default_window_icon().cloned() {
                builder = builder.icon(icon);
            }
            let _tray = builder
                .menu(&menu)
                .show_menu_on_left_click(true)
                .tooltip("LiteView")
                .on_menu_event(|app, event| {
                    match event.id.as_ref() {
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
                    }
                })
                .build(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
