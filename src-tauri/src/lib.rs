pub mod commands;
pub mod core;
pub mod error;
pub mod models;
pub mod platform;
pub mod security;
pub mod state;
pub mod tray;

use crate::core::launcher::{
    DEFAULT_LAUNCHER_SHORTCUT, LAUNCHER_WINDOW_LABEL, LauncherShortcutState,
};
use crate::core::{HISTORY_FILE_NAME, LaunchHistoryStore};
use state::AppState;
use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState::new("DeskPilot", env!("CARGO_PKG_VERSION"));

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = tray::show_main_window(app);
        }))
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .manage(app_state)
        .manage(LauncherShortcutState::new())
        // Registered up front so history commands never hit unmanaged state; the file is
        // attached in `setup` once the app-data folder is known.
        .manage(LaunchHistoryStore::in_memory())
        .on_window_event(|window, event| match event {
            tauri::WindowEvent::CloseRequested { api, .. } => {
                let state = window.state::<AppState>();
                if window.label() == LAUNCHER_WINDOW_LABEL {
                    // The launcher is reused for every shortcut press: hide it instead of
                    // destroying it, except while the app is quitting.
                    if !state.is_quitting() {
                        let _ = window.hide();
                        api.prevent_close();
                    }
                } else {
                    tray::handle_close_requested(&state, window, api);
                }
            }
            tauri::WindowEvent::Destroyed if window.label() == tray::MAIN_WINDOW_LABEL => {
                // Without this, the hidden launcher window would keep the process alive
                // after the main window closes when no tray icon is available.
                let state = window.state::<AppState>();
                if !state.is_quitting() {
                    state.set_quitting(true);
                    window.app_handle().exit(0);
                }
            }
            _ => {}
        })
        .setup(|app| {
            // Without an app-data folder, history stays in memory for this session only.
            if let Ok(dir) = app.path().app_data_dir() {
                let _ = app
                    .state::<LaunchHistoryStore>()
                    .attach_file(dir.join(HISTORY_FILE_NAME));
            }
            // Global shortcut for the quick launcher. A conflict with another app is
            // reported in the UI instead of failing startup.
            let registration = app.global_shortcut().on_shortcut(
                DEFAULT_LAUNCHER_SHORTCUT,
                |app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        let _ = commands::launcher::toggle_launcher(app);
                    }
                },
            );
            let shortcut_state = app.state::<LauncherShortcutState>();
            match registration {
                Ok(()) => shortcut_state.set_registered(),
                Err(_) => shortcut_state.set_failed(),
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
            commands::launcher::list_applications,
            commands::launcher::hide_launcher,
            commands::launcher::show_main_window,
            commands::launcher::get_launcher_shortcut,
            commands::processes::list_processes,
            commands::system::get_system_metrics
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
