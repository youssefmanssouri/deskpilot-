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
        .plugin(tauri_plugin_opener::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            commands::get_app_status,
            commands::validate_command_intent,
            commands::discover_applications
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
