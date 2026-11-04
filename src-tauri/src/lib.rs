pub mod commands;
pub mod core;
pub mod error;
pub mod models;
pub mod platform;
pub mod security;
pub mod state;
pub mod tray;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState::new("DeskPilot", "0.1.0");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = tray::show_main_window(app);
        }))
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let state = window.state::<AppState>();
                tray::handle_close_requested(&state, window, api);
            }
        })
        .setup(|app| {
            tray::setup_tray(app)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::status::get_app_status,
            commands::status::validate_command_intent,
            commands::applications::discover_applications,
            commands::applications::launch_application,
            commands::processes::list_processes,
            commands::system::get_system_metrics
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
