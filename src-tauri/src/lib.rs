use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    Emitter, Manager,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let start_capture_i =
                MenuItem::with_id(app, "start_capture", "Select screen / Start capture", true, None::<&str>)?;
            let stop_capture_i = MenuItem::with_id(app, "stop_capture", "Stop capture", true, None::<&str>)?;
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
                    }
                })
                .build(app)?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
