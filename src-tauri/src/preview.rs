//! Native preview window using minifb (uses X11 directly on Linux, no GTK conflict).
//! Uses shared state instead of channel to ensure we always show the latest frame.

use minifb::{Window, WindowOptions};
use std::sync::{Arc, Mutex};

/// Frame data for preview - producer overwrites, consumer takes.
pub struct FrameData {
    pub width: u32,
    pub height: u32,
    pub buffer: Vec<u32>,
}

/// Shared state between capture thread and preview thread.
/// Producer sets `frame` to Some(...), consumer takes it (sets to None).
/// `running` is set to false to signal preview thread to exit.
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

/// Runs the native preview window. Call from a dedicated thread.
/// Polls `state.frame` for new frames; exits when `state.running` is false.
pub fn run_preview_window(state: Arc<PreviewState>) {
    use std::sync::atomic::Ordering;
    
    let mut window: Option<Window> = None;
    let mut current_size: (usize, usize) = (0, 0);

    while state.running.load(Ordering::Relaxed) {
        // Take the frame if available (non-blocking)
        let frame_data = state.frame.lock().unwrap().take();

        if let Some(FrameData { width, height, buffer }) = frame_data {
            let w = width as usize;
            let h = height as usize;

            // Create or recreate window if size changed
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

            // Update window with frame buffer
            if let Some(ref mut win) = window {
                if win.is_open() {
                    let _ = win.update_with_buffer(&buffer, w, h);
                } else {
                    break; // Window closed by user
                }
            }
        } else {
            // No new frame - just update window (process events) and yield
            if let Some(ref mut win) = window {
                if !win.is_open() {
                    break;
                }
                win.update();
            }
            // Small sleep to avoid busy-waiting when no frames
            std::thread::sleep(std::time::Duration::from_micros(500));
        }
    }
}
