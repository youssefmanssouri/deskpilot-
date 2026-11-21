pub mod commands;
pub mod core;
pub mod error;
pub mod models;
pub mod platform;
pub mod security;
pub mod state;
pub mod tray;

use crate::commands::launcher::register_launcher_shortcut;
use crate::commands::settings::GlassState;
use crate::core::launcher::{LAUNCHER_WINDOW_LABEL, LauncherShortcutState};
use crate::core::settings::{SETTINGS_FILE_NAME, Settings, SettingsStore, supports_glass};
use crate::core::{HISTORY_FILE_NAME, LaunchHistoryStore};
use state::AppState;
use std::sync::atomic::Ordering;
use tauri::window::{Effect, EffectsBuilder};
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

/// Argument the "Start with Windows" entry passes, so DeskPilot can start in the tray at login.
pub const AUTOSTART_ARG: &str = "--autostart";

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
        // State that commands use is registered up front so a command can never hit
        // unmanaged state; files are attached in `setup` once the app-data folder is known.
        .manage(LaunchHistoryStore::in_memory())
        .manage(SettingsStore::in_memory())
        .manage(GlassState::default())
        .manage(crate::core::AppIconCache::new())
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
            // 1. Settings and history. Without an app-data folder both stay in memory.
            let data_dir = app.path().app_data_dir().ok();
            let settings = match &data_dir {
                Some(dir) => app
                    .state::<SettingsStore>()
                    .attach_file(dir.join(SETTINGS_FILE_NAME))
                    .unwrap_or_default(),
                None => Settings::default(),
            };
            let history = app.state::<LaunchHistoryStore>();
            if let Some(dir) = &data_dir {
                let _ = history.attach_file(dir.join(HISTORY_FILE_NAME));
            }
            history.set_enabled(settings.history_enabled);

            // 2. Main window. Built here instead of tauri.conf.json because transparency
            //    (needed for Mica) can only be chosen when a window is created. Glass is only
            //    used on Windows 11; elsewhere a transparent window would be see-through.
            #[cfg(target_os = "windows")]
            let build = platform::windows_build_number();
            #[cfg(not(target_os = "windows"))]
            let build = None;
            let glass_supported = supports_glass(build);
            let glass_active = settings.glass_effect && glass_supported;
            let glass = app.state::<GlassState>();
            glass.supported.store(glass_supported, Ordering::Relaxed);
            glass.active.store(glass_active, Ordering::Relaxed);

            let mut main = WebviewWindowBuilder::new(
                app,
                tray::MAIN_WINDOW_LABEL,
                WebviewUrl::App("index.html".into()),
            )
            .title("DeskPilot")
            .inner_size(960.0, 680.0)
            .min_inner_size(520.0, 450.0);
            if glass_active {
                main = main
                    .transparent(true)
                    .effects(EffectsBuilder::new().effect(Effect::Mica).build());
            }
            main.build()?;

            // 3. Launcher shortcut from settings. A conflict is reported in the UI
            //    instead of failing startup.
            let shortcut = app.state::<LauncherShortcutState>();
            match register_launcher_shortcut(app.handle(), &settings.launcher_shortcut) {
                Ok(()) => shortcut.set_registered(&settings.launcher_shortcut),
                Err(_) => shortcut.set_failed(&settings.launcher_shortcut),
            }

            // 4. Tray, then start hidden if Windows launched DeskPilot at login. Only hide
            //    when the tray exists, so the app is never left without a visible window.
            tray::init_tray_lifecycle(app)?;
            let started_at_login = std::env::args().any(|a| a == AUTOSTART_ARG);
            if started_at_login && app.state::<AppState>().is_tray_available() {
                let _ = tray::hide_main_window(app.handle());
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::status::get_app_status,
            commands::status::validate_command_intent,
            commands::status::is_running_elevated,
            commands::applications::discover_applications,
            commands::applications::launch_application,
            commands::icons::get_application_icon,
            commands::history::get_launch_history,
            commands::history::clear_launch_history,
            commands::launcher::list_applications,
            commands::launcher::hide_launcher,
            commands::launcher::show_main_window,
            commands::launcher::get_launcher_shortcut,
            commands::settings::get_settings,
            commands::settings::set_theme,
            commands::settings::set_history_enabled,
            commands::settings::set_launcher_shortcut,
            commands::settings::set_start_with_windows,
            commands::settings::set_glass_effect,
            commands::processes::list_processes,
            commands::system::get_system_metrics
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
