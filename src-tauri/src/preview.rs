use minifb::{Window, WindowOptions};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

pub struct FrameData {
    pub width: u32,
    pub height: u32,
    pub buffer: Vec<u32>,
}

pub struct PreviewState {
    pub frame: Mutex<Option<FrameData>>,
    pub running: std::sync::atomic::AtomicBool,
}

impl Default for PreviewState {
    fn default() -> Self {
        Self {
            frame: Mutex::new(None),
            running: std::sync::atomic::AtomicBool::new(true),
        }
    }
}

pub fn run_preview_window(state: Arc<PreviewState>) {
    let mut window: Option<Window> = None;
    let mut current_size: (usize, usize) = (0, 0);
    let wait_duration = std::time::Duration::from_micros(500);

    while state.running.load(Ordering::Relaxed) {
        let frame_data = state.frame.lock().unwrap().take();

        if let Some(FrameData { width, height, buffer }) = frame_data {
            let w = width as usize;
            let h = height as usize;

            if window.is_none() || current_size != (w, h) {
                window = Window::new(
                    "LiteView Preview",
                    w,
                    h,
                    WindowOptions {
                        resize: true,
                        topmost: true,
                        borderless: true,
                        ..Default::default()
                    },
                )
                .ok();
                current_size = (w, h);
            }

            if let Some(ref mut win) = window {
                if !win.is_open() {
                    break;
                }
                let _ = win.update_with_buffer(&buffer, w, h);
            }
        } else {
            if let Some(ref mut win) = window {
                if !win.is_open() {
                    break;
                }
                win.update();
            }
            std::thread::sleep(wait_duration);
        }
    }
}
