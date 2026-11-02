pub mod commands;
pub mod core;
pub mod error;
pub mod models;
pub mod platform;
pub mod security;
pub mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState::new("DeskPilot", "0.1.0");

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            use tauri::Manager;
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::status::get_app_status,
            commands::status::validate_command_intent,
            commands::applications::discover_applications,
            commands::applications::launch_application
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
