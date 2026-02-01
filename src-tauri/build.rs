fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(
            tauri_build::AppManifest::new()
                .commands(&["get_scap_targets", "start_scap_capture", "stop_scap_capture"]),
        ),
    )
    .expect("failed to run tauri-build");
}
