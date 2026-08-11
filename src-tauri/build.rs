fn main() -> Result<(), Box<dyn std::error::Error>> {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "get_local_today",
            "get_daily_page",
            "save_daily_page",
            "list_notes",
            "create_note",
            "update_note",
            "delete_note",
        ]),
    ))?;

    Ok(())
}
