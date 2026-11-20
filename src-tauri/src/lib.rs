pub mod commands;
pub mod core;
pub mod error;
pub mod models;
pub mod platform;
pub mod security;
pub mod state;
pub mod tray;

use crate::core::{HISTORY_FILE_NAME, LaunchHistoryStore};
use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState::new("DeskPilot", env!("CARGO_PKG_VERSION"));

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = tray::show_main_window(app);
        }))
        .manage(app_state)
        // Registered up front so history commands never hit unmanaged state; the file is
        // attached in `setup` once the app-data folder is known.
        .manage(LaunchHistoryStore::in_memory())
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<AppState>();
                tray::handle_close_requested(&state, window, api);
            }
        })
        .setup(|app| {
            // Without an app-data folder, history stays in memory for this session only.
            if let Ok(dir) = app.path().app_data_dir() {
                let _ = app
                    .state::<LaunchHistoryStore>()
                    .attach_file(dir.join(HISTORY_FILE_NAME));
            }
            tray::init_tray_lifecycle(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::status::get_app_status,
            commands::status::validate_command_intent,
            commands::applications::discover_applications,
            commands::applications::launch_application,
            commands::history::get_launch_history,
            commands::history::clear_launch_history,
            commands::processes::list_processes,
            commands::system::get_system_metrics
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
