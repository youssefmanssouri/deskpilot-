use crate::commands::launcher::{register_launcher_shortcut, unregister_launcher_shortcut};
use crate::core::LaunchHistoryStore;
use crate::core::launcher::LauncherShortcutState;
use crate::core::settings::{SettingsStore, ThemePreference, normalize_shortcut, shortcut_label};
use crate::error::IpcErrorDto;
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_autostart::ManagerExt;

/// Event broadcast to all windows after a setting changes (e.g. the launcher re-applies the theme).
pub const SETTINGS_CHANGED_EVENT: &str = "deskpilot://settings-changed";

/// Whether the glass effect is possible on this PC and whether it is on in this session.
/// Set once in `setup`; changing the setting only takes effect after a restart.
#[derive(Debug, Default)]
pub struct GlassState {
    pub supported: AtomicBool,
    pub active: AtomicBool,
}

/// Everything the Settings page shows.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsDto {
    pub theme: ThemePreference,
    pub history_enabled: bool,
    /// Canonical form, e.g. "Ctrl+Super+K".
    pub launcher_shortcut: String,
    /// Display form, e.g. "Ctrl+Win+K".
    pub launcher_shortcut_label: String,
    pub shortcut_registered: bool,
    pub shortcut_message: Option<String>,
    /// Read from Windows each time; `None` if it could not be checked.
    pub start_with_windows: Option<bool>,
    /// The saved preference.
    pub glass_effect: bool,
    /// True on Windows 11.
    pub glass_supported: bool,
    /// True if the glass effect is on in this session.
    pub glass_active: bool,
}

fn internal(message: &str) -> IpcErrorDto {
    IpcErrorDto {
        code: "OPERATION_FAILED".to_string(),
        message: message.to_string(),
        details: None,
    }
}

fn build_dto<R: Runtime>(app: &AppHandle<R>) -> Result<SettingsDto, IpcErrorDto> {
    let settings = app
        .state::<SettingsStore>()
        .get()
        .map_err(IpcErrorDto::from)?;
    let shortcut = app
        .state::<LauncherShortcutState>()
        .status()
        .map_err(IpcErrorDto::from)?;
    let glass = app.state::<GlassState>();
    Ok(SettingsDto {
        theme: settings.theme,
        history_enabled: settings.history_enabled,
        launcher_shortcut_label: shortcut_label(&settings.launcher_shortcut),
        launcher_shortcut: settings.launcher_shortcut,
        shortcut_registered: shortcut.registered,
        shortcut_message: shortcut.message,
        start_with_windows: app.autolaunch().is_enabled().ok(),
        glass_effect: settings.glass_effect,
        glass_supported: glass.supported.load(Ordering::Relaxed),
        glass_active: glass.active.load(Ordering::Relaxed),
    })
}

fn notify<R: Runtime>(app: &AppHandle<R>) -> Result<SettingsDto, IpcErrorDto> {
    let dto = build_dto(app)?;
    let _ = app.emit(SETTINGS_CHANGED_EVENT, dto.clone());
    Ok(dto)
}

#[tauri::command]
pub fn get_settings<R: Runtime>(app: AppHandle<R>) -> Result<SettingsDto, IpcErrorDto> {
    build_dto(&app)
}

#[tauri::command]
pub fn set_theme<R: Runtime>(
    app: AppHandle<R>,
    theme: ThemePreference,
) -> Result<SettingsDto, IpcErrorDto> {
    app.state::<SettingsStore>()
        .update(|s| s.theme = theme)
        .map_err(IpcErrorDto::from)?;
    notify(&app)
}

/// Turning history off also clears it, as the Settings page says.
#[tauri::command]
pub fn set_history_enabled<R: Runtime>(
    app: AppHandle<R>,
    enabled: bool,
) -> Result<SettingsDto, IpcErrorDto> {
    app.state::<SettingsStore>()
        .update(|s| s.history_enabled = enabled)
        .map_err(IpcErrorDto::from)?;
    let history = app.state::<LaunchHistoryStore>();
    history.set_enabled(enabled);
    if !enabled {
        history.clear().map_err(IpcErrorDto::from)?;
    }
    notify(&app)
}

/// Swaps the launcher shortcut. The new shortcut is validated first; if Windows refuses it
/// (taken by another app or reserved), the previous one is restored and an error explains why.
#[tauri::command]
pub fn set_launcher_shortcut<R: Runtime>(
    app: AppHandle<R>,
    shortcut: String,
) -> Result<SettingsDto, IpcErrorDto> {
    let next = normalize_shortcut(&shortcut).map_err(IpcErrorDto::from)?;
    let store = app.state::<SettingsStore>();
    let status = app.state::<LauncherShortcutState>();
    let current = store.get().map_err(IpcErrorDto::from)?.launcher_shortcut;

    let already_active = status.status().map(|s| s.registered).unwrap_or(false);
    if next == current && already_active {
        return build_dto(&app);
    }

    unregister_launcher_shortcut(&app, &current);
    if register_launcher_shortcut(&app, &next).is_err() {
        // Keep the launcher reachable with the previous shortcut.
        match register_launcher_shortcut(&app, &current) {
            Ok(()) => status.set_registered(&current),
            Err(_) => status.set_failed(&current),
        }
        return Err(IpcErrorDto {
            code: "UNAVAILABLE".to_string(),
            message: format!(
                "{} is already used by another app or reserved by Windows. Your shortcut is still {}.",
                shortcut_label(&next),
                shortcut_label(&current)
            ),
            details: None,
        });
    }
    status.set_registered(&next);
    store
        .update(|s| s.launcher_shortcut = next.clone())
        .map_err(IpcErrorDto::from)?;
    notify(&app)
}

/// Adds or removes DeskPilot's entry in the Windows startup list (HKCU Run key).
/// When Windows starts DeskPilot, it opens in the tray instead of showing the window.
#[tauri::command]
pub fn set_start_with_windows<R: Runtime>(
    app: AppHandle<R>,
    enabled: bool,
) -> Result<SettingsDto, IpcErrorDto> {
    let result = if enabled {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    };
    result.map_err(|_| {
        internal(if enabled {
            "DeskPilot could not be added to the Windows startup list."
        } else {
            "DeskPilot could not be removed from the Windows startup list."
        })
    })?;
    notify(&app)
}

/// Saves the glass preference. It takes effect the next time DeskPilot starts, and only
/// on Windows 11.
#[tauri::command]
pub fn set_glass_effect<R: Runtime>(
    app: AppHandle<R>,
    enabled: bool,
) -> Result<SettingsDto, IpcErrorDto> {
    app.state::<SettingsStore>()
        .update(|s| s.glass_effect = enabled)
        .map_err(IpcErrorDto::from)?;
    notify(&app)
}
