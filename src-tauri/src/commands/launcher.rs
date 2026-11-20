use crate::core::launcher::{
    LAUNCHER_SHOWN_EVENT, LAUNCHER_WINDOW_LABEL, LauncherShortcutState, NAVIGATE_EVENT,
    ShortcutStatus, validate_navigate_request,
};
use crate::error::IpcErrorDto;
use crate::models::ApplicationDto;
use crate::state::AppState;
use crate::tray::{self, MAIN_WINDOW_LABEL};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

/// Shows the launcher centred and focused, or hides it if it is already visible.
/// Called by the global shortcut handler.
pub fn toggle_launcher<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<()> {
    let Some(window) = app.get_webview_window(LAUNCHER_WINDOW_LABEL) else {
        return Ok(());
    };
    if window.is_visible()? {
        window.hide()?;
    } else {
        window.center()?;
        window.show()?;
        window.set_focus()?;
        // Lets the page reset its query and refresh data each time it opens.
        window.emit(LAUNCHER_SHOWN_EVENT, ())?;
    }
    Ok(())
}

fn hide_launcher_window<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(LAUNCHER_WINDOW_LABEL) {
        let _ = window.hide();
    }
}

/// Returns the current in-memory catalog without rescanning. The launcher calls this
/// every time it opens, so it must stay cheap; an empty list means discovery has not
/// run yet and the caller should run `discover_applications` once.
#[tauri::command]
pub fn list_applications(state: State<'_, AppState>) -> Result<Vec<ApplicationDto>, IpcErrorDto> {
    let (_, apps) = state.catalog_snapshot().map_err(IpcErrorDto::from)?;
    Ok(apps.into_iter().map(ApplicationDto::from).collect())
}

/// Hides the launcher window (Esc, focus loss, or after a successful launch).
#[tauri::command]
pub fn hide_launcher<R: Runtime>(app: AppHandle<R>) {
    hide_launcher_window(&app);
}

/// Brings the main window to the front on a given view, optionally selecting an app,
/// and hides the launcher. The request is validated because it comes from a page.
#[tauri::command]
pub fn show_main_window<R: Runtime>(
    app: AppHandle<R>,
    view: String,
    application_id: Option<String>,
) -> Result<(), IpcErrorDto> {
    let request =
        validate_navigate_request(&view, application_id.as_deref()).map_err(IpcErrorDto::from)?;
    hide_launcher_window(&app);
    tray::show_main_window(&app).map_err(|_| IpcErrorDto {
        code: "UNAVAILABLE".to_string(),
        message: "The DeskPilot window could not be shown.".to_string(),
        details: None,
    })?;
    app.emit_to(MAIN_WINDOW_LABEL, NAVIGATE_EVENT, request)
        .map_err(|_| IpcErrorDto {
            code: "OPERATION_FAILED".to_string(),
            message: "The DeskPilot window could not be updated.".to_string(),
            details: None,
        })
}

/// Reports whether the global shortcut is active, so the UI can explain a conflict.
#[tauri::command]
pub fn get_launcher_shortcut(
    shortcut: State<'_, LauncherShortcutState>,
) -> Result<ShortcutStatus, IpcErrorDto> {
    shortcut.status().map_err(IpcErrorDto::from)
}
