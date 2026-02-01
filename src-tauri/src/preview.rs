use minifb::{Window, WindowOptions};
use std::sync::atomic::Ordering;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

const EVENT_POLL_INTERVAL_MS: u64 = 16;

pub struct FrameData {
    pub width: u32,
    pub height: u32,
    pub buffer: Vec<u32>,
}

pub struct PreviewState {
    pub frame: Mutex<Option<FrameData>>,
    pub running: std::sync::atomic::AtomicBool,
    pub frame_consumed: Condvar,
    pub frame_available: Condvar,
}

impl Default for PreviewState {
    fn default() -> Self {
        Self {
            frame: Mutex::new(None),
            running: std::sync::atomic::AtomicBool::new(true),
            frame_consumed: Condvar::new(),
            frame_available: Condvar::new(),
        }
    }
}

pub fn run_preview_window(state: Arc<PreviewState>) {
    let mut window: Option<Window> = None;
    let mut current_size: (usize, usize) = (0, 0);
    let poll_timeout = Duration::from_millis(EVENT_POLL_INTERVAL_MS);

    while state.running.load(Ordering::Relaxed) {
        let frame_data = {
            let mut guard = state.frame.lock().unwrap();
            loop {
                if let Some(data) = guard.take() {
                    state.frame_consumed.notify_one();
                    break Some(data);
                }
                let (g, timed_out) =
                    state.frame_available.wait_timeout(guard, poll_timeout).unwrap();
                guard = g;
                if !state.running.load(Ordering::Relaxed) {
                    return;
                }
                if guard.is_none() && timed_out.timed_out() {
                    drop(guard);
                    if let Some(ref mut win) = window {
                        if !win.is_open() {
                            return;
                        }
                        win.update();
                    }
                    guard = state.frame.lock().unwrap();
                }
            }
        };

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
        }
    }
}
