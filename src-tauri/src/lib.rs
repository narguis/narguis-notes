use tauri::{Emitter, Manager};

pub mod civil_date;
pub mod commands;
pub mod dto;
pub mod error;
pub mod ipc;
mod package_self_test;
pub mod storage;

pub const LOCAL_TODAY_CHANGED_EVENT: &str = "planner://local-today-changed";
pub const PACKAGE_SELF_TEST_SUCCESS_LOG: &str =
    "PACKAGE_SELF_TEST_OK title_description_template_mapping_after_restart";

pub fn run() {
    let application = tauri::Builder::default()
        .setup(|app| {
            let app_data_directory = app.path().app_data_dir()?;
            app.manage(storage::Storage::open_in_app_data_directory(
                &app_data_directory,
            )?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_local_today,
            commands::get_daily_page,
            commands::save_daily_page,
            commands::list_notes,
            commands::create_note,
            commands::update_note,
            commands::delete_note,
            commands::list_planner_lines,
            commands::create_planner_line,
            commands::update_planner_line,
            commands::delete_planner_line,
            commands::move_planner_line,
            commands::set_planner_line_collapsed,
            commands::set_planner_line_time,
            commands::list_task_templates,
            commands::create_task_template,
            commands::update_task_template,
            commands::delete_task_template,
            commands::insert_task_template_copy,
        ])
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::Focused(true)) {
                emit_local_today(window.app_handle());
            }
        })
        .build(tauri::generate_context!());

    match application {
        Ok(application) => application.run(|app_handle, event| {
            if matches!(event, tauri::RunEvent::Resumed) {
                emit_local_today(app_handle);
            }
        }),
        Err(error) => {
            eprintln!("failed to run Narguis Notes App: {error}");
            std::process::exit(1);
        }
    }
}

pub fn run_package_self_test(app_data_directory: &std::path::Path) -> Result<(), error::AppError> {
    package_self_test::run(app_data_directory)
}

fn emit_local_today(app_handle: &tauri::AppHandle) {
    if let Ok(today) = civil_date::CivilDate::today() {
        let _ = app_handle.emit(LOCAL_TODAY_CHANGED_EVENT, today.as_str());
    }
}
