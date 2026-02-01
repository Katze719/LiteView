fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(
            tauri_build::AppManifest::new()
                .commands(&[
            "get_capture_targets",
            "get_capture_settings",
            "set_capture_settings",
            "start_capture",
            "stop_capture",
        ]),
        ),
    )
    .expect("failed to run tauri-build");
}
