mod preview;

use preview::{FrameData, PreviewState};
use scap::capturer::{Capturer, Options, Resolution as ScapResolution};
use scap::frame::{Frame, FrameType};
use scap::{get_all_targets, has_permission, is_supported, request_permission, Target};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::TrayIconBuilder,
};
use tauri::{AppHandle, Emitter, Manager, State};

const DEFAULT_CAPTURE_FPS: u32 = 60;
const DEFAULT_RESOLUTION: &str = "captured";
const SETTINGS_FILENAME: &str = "settings.json";

fn settings_path(app: &AppHandle) -> Option<PathBuf> {
    app.path().app_data_dir().ok().map(|p| p.join(SETTINGS_FILENAME))
}

fn load_settings_from_disk(app: &AppHandle) -> Option<CaptureSettings> {
    let path = settings_path(app)?;
    let contents = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&contents).ok()
}

fn save_settings_to_disk(app: &AppHandle, settings: &CaptureSettings) -> Result<(), String> {
    let path = settings_path(app).ok_or("App data dir not available")?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let contents = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(&path, contents).map_err(|e| e.to_string())
}

fn default_show_cursor() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CaptureSettings {
    fps: u32,
    resolution: String,
    #[serde(default)]
    target_index: Option<usize>,
    #[serde(default = "default_show_cursor")]
    show_cursor: bool,
}

impl Default for CaptureSettings {
    fn default() -> Self {
        Self {
            fps: DEFAULT_CAPTURE_FPS,
            resolution: DEFAULT_RESOLUTION.to_string(),
            target_index: None,
            show_cursor: true,
        }
    }
}

fn resolution_from_str(s: &str) -> ScapResolution {
    match s {
        "480p" => ScapResolution::_480p,
        "720p" => ScapResolution::_720p,
        "1080p" => ScapResolution::_1080p,
        "1440p" => ScapResolution::_1440p,
        "2160p" => ScapResolution::_2160p,
        "4320p" => ScapResolution::_4320p,
        _ => ScapResolution::Captured,
    }
}

/// Target (width, height) for a resolution string and source aspect ratio (width/height).
/// Returns None for "captured" (no scaling).
fn resolution_target_size(resolution: &str, aspect_ratio: f32) -> Option<(u32, u32)> {
    let (base_w, base_h) = match resolution.to_lowercase().as_str() {
        "480p" => (640, (640_f32 / aspect_ratio).floor() as u32),
        "720p" => (1280, (1280_f32 / aspect_ratio).floor() as u32),
        "1080p" => (1920, (1920_f32 / aspect_ratio).floor() as u32),
        "1440p" => (2560, (2560_f32 / aspect_ratio).floor() as u32),
        "2160p" => (3840, (3840_f32 / aspect_ratio).floor() as u32),
        "4320p" => (7680, (7680_f32 / aspect_ratio).floor() as u32),
        _ => return None,
    };
    Some((base_w, base_h.max(1)))
}

/// Nearest-neighbor resize of RGBA32 buffer (packed u32: 0xAABBGGRR or similar).
fn resize_frame(
    src_w: u32,
    src_h: u32,
    src: &[u32],
    dst_w: u32,
    dst_h: u32,
) -> Vec<u32> {
    let mut dst = vec![0u32; (dst_w as usize).saturating_mul(dst_h as usize)];
    if src_w == 0 || src_h == 0 || dst_w == 0 || dst_h == 0 {
        return dst;
    }
    let src_w = src_w as usize;
    let src_h = src_h as usize;
    let dst_w = dst_w as usize;
    let dst_h = dst_h as usize;
    for y in 0..dst_h {
        let sy = (y as u64 * (src_h as u64 - 1) / dst_h.max(1) as u64) as usize;
        let src_row = sy.saturating_mul(src_w);
        for x in 0..dst_w {
            let sx = (x as u64 * (src_w as u64 - 1) / dst_w.max(1) as u64) as usize;
            dst[y * dst_w + x] = src.get(src_row + sx).copied().unwrap_or(0);
        }
    }
    dst
}

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
    settings: Arc<Mutex<CaptureSettings>>,
}

impl Default for CaptureState {
    fn default() -> Self {
        Self {
            stop_requested: Arc::new(AtomicBool::new(false)),
            preview_state: Arc::new(Mutex::new(None)),
            settings: Arc::new(Mutex::new(CaptureSettings::default())),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct CaptureSettingsDto {
    fps: u32,
    resolution: String,
    target_index: Option<usize>,
    show_cursor: bool,
}

#[tauri::command]
fn get_capture_settings(state: State<CaptureState>) -> CaptureSettingsDto {
    let s = state.settings.lock().unwrap();
    CaptureSettingsDto {
        fps: s.fps,
        resolution: s.resolution.clone(),
        target_index: s.target_index,
        show_cursor: s.show_cursor,
    }
}

#[tauri::command]
fn set_capture_settings(
    app: AppHandle,
    fps: u32,
    resolution: String,
    target_index: Option<usize>,
    show_cursor: bool,
    state: State<CaptureState>,
) -> Result<(), String> {
    let fps = fps.clamp(1, 120);
    let resolution = resolution.to_lowercase();
    let valid = ["captured", "480p", "720p", "1080p", "1440p", "2160p", "4320p"];
    if !valid.contains(&resolution.as_str()) {
        return Err(format!("Invalid resolution: {}", resolution));
    }
    let settings = CaptureSettings {
        fps,
        resolution: resolution.clone(),
        target_index,
        show_cursor,
    };
    *state.settings.lock().unwrap() = settings.clone();
    save_settings_to_disk(&app, &settings)?;
    Ok(())
}

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
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

    let settings = state.settings.lock().unwrap().clone();
    let resolution_for_scale = settings.resolution.clone();
    let target_fps = settings.fps.max(1);
    let target_index_for_thread = target_index.or(settings.target_index);

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

    let stop_requested_clone = state.stop_requested.clone();

    thread::spawn(move || {
        let target = target_index_for_thread.and_then(|idx| get_all_targets().into_iter().nth(idx));
        let options = Options {
            fps: settings.fps,
            show_cursor: settings.show_cursor,
            show_highlight: false,
            target,
            crop_area: None,
            output_type: FrameType::BGRAFrame,
            output_resolution: resolution_from_str(&settings.resolution),
            excluded_targets: None,
            ..Default::default()
        };
        let mut capturer = match Capturer::build(options) {
            Ok(c) => c,
            Err(e) => {
                let _ = app_handle.emit("capture-error", e.to_string());
                preview_state.running.store(false, Ordering::Relaxed);
                return;
            }
        };
        capturer.start_capture();

        let frame_interval = Duration::from_secs_f64(1.0 / target_fps as f64);
        let mut last_push = Instant::now();

        while !stop_requested_clone.load(Ordering::Relaxed)
            && preview_state.running.load(Ordering::Relaxed)
        {
            let frame = match capturer.get_next_frame() {
                Ok(f) => f,
                Err(_) => break,
            };

            if let Some((width, height, buffer)) = frame_to_buffer(&frame) {
                let now = Instant::now();
                if now.duration_since(last_push) < frame_interval {
                    continue;
                }
                last_push = now;

                let (out_width, out_height, out_buffer) =
                    if let Some((tw, th)) =
                        resolution_target_size(&resolution_for_scale, width as f32 / height as f32)
                    {
                        let scaled = resize_frame(width, height, &buffer, tw, th);
                        (tw, th, scaled)
                    } else {
                        (width, height, buffer)
                    };
                *preview_state.frame.lock().unwrap() = Some(FrameData {
                    width: out_width,
                    height: out_height,
                    buffer: out_buffer,
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
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::UI::HiDpi::{
            SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
        };
        let _ = unsafe { SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) };
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(CaptureState::default())
        .invoke_handler(tauri::generate_handler![
            get_app_version,
            get_capture_targets,
            get_capture_settings,
            set_capture_settings,
            start_capture,
            stop_capture,
        ])
        .setup(|app| {
            if let Some(loaded) = load_settings_from_disk(&app.handle()) {
                *app.state::<CaptureState>().settings.lock().unwrap() = loaded;
            }
            let slot = app.state::<CaptureState>().preview_state.clone();
            thread::spawn(move || preview::run_preview_window(slot));

            let start_capture_i = MenuItem::with_id(
                app,
                "start_capture",
                "Start capture…",
                true,
                None::<&str>,
            )?;
            let stop_capture_i =
                MenuItem::with_id(app, "stop_capture", "Stop capture", true, None::<&str>)?;
            let sep1 = PredefinedMenuItem::separator(app)?;
            let show_i = MenuItem::with_id(app, "show", "Show window", true, None::<&str>)?;
            let settings_i = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
            let sep2 = PredefinedMenuItem::separator(app)?;
            let quit_i = MenuItem::with_id(app, "quit", "Quit LiteView", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[
                    &start_capture_i,
                    &stop_capture_i,
                    &sep1,
                    &show_i,
                    &settings_i,
                    &sep2,
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
