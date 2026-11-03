use crate::state::AppState;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, CloseRequestApi, Manager, Runtime};

/// Tray menu action identifiers.
pub const TRAY_ID_SHOW: &str = "tray_show";
pub const TRAY_ID_HIDE: &str = "tray_hide";
pub const TRAY_ID_QUIT: &str = "tray_quit";

/// Tray icon tooltip.
pub const TRAY_TOOLTIP: &str = "DeskPilot";

/// Main window identifier as defined in `tauri.conf.json`.
pub const MAIN_WINDOW_LABEL: &str = "main";

/// Strongly-typed tray actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    Show,
    Hide,
    Quit,
}

impl TrayAction {
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            TRAY_ID_SHOW => Some(Self::Show),
            TRAY_ID_HIDE => Some(Self::Hide),
            TRAY_ID_QUIT => Some(Self::Quit),
            _ => None,
        }
    }
}

/// Restores, unminimizes, and focuses the existing main window.
///
/// If the main window is currently hidden, calling `show()` brings it
/// back to visibility, while `unminimize()` and `set_focus()` guarantee
/// it surfaces in the foreground.
pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        window.show().map_err(|e| e.to_string())?;
        window.unminimize().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err(format!("Main window '{MAIN_WINDOW_LABEL}' not found"))
    }
}

/// Hides the main window without terminating the application.
pub fn hide_main_window<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        window.hide().map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err(format!("Main window '{MAIN_WINDOW_LABEL}' not found"))
    }
}

/// Fully terminates the application.
///
/// Sets the `is_quitting` atomic flag in `AppState` before requesting application exit,
/// ensuring that window close events during shutdown do not trigger hide-to-tray.
pub fn quit_app<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppState>();
    state.set_quitting(true);
    app.exit(0);
}

/// Dispatches a tray menu event to the corresponding lifecycle action.
pub fn handle_tray_menu_action<R: Runtime>(app: &AppHandle<R>, menu_id: &str) {
    match TrayAction::from_id(menu_id) {
        Some(TrayAction::Show) => {
            let _ = show_main_window(app);
        }
        Some(TrayAction::Hide) => {
            let _ = hide_main_window(app);
        }
        Some(TrayAction::Quit) => {
            quit_app(app);
        }
        None => {}
    }
}

/// Evaluates whether a close request should be intercepted and converted to hide-to-tray.
///
/// Returns `true` if the application is in normal runtime mode (hide to tray).
/// Returns `false` if an explicit quit has been triggered (allow termination).
pub fn should_prevent_close(state: &AppState) -> bool {
    !state.is_quitting()
}

/// Intercepts window close requests to implement predictable hide-to-tray behavior.
///
/// If the application is not actively quitting, closing the window hides it and
/// cancels the close request. If an explicit quit was initiated, the close request
/// is allowed to proceed without interference.
pub fn handle_close_requested(
    state: &AppState,
    window: &tauri::Window<impl Runtime>,
    api: &CloseRequestApi,
) {
    if should_prevent_close(state) {
        let _ = window.hide();
        api.prevent_close();
    }
}

/// Resolves the tray icon image from bundle resources, native Windows icon resource,
/// or a fallback transparent RGBA buffer.
pub fn resolve_tray_icon<R: Runtime>(app: &AppHandle<R>) -> tauri::image::Image<'static> {
    app.default_window_icon()
        .map(|icon| icon.clone().to_owned())
        .or_else(|| tauri::image::Image::from_app_icon_resource(32).ok())
        .unwrap_or_else(|| {
            // Safe fallback for test environments or headless runs
            tauri::image::Image::new_owned(vec![0u8; 16 * 16 * 4], 16, 16)
        })
}

/// Constructs and mounts the native system tray icon with context menu.
pub fn setup_tray<R: Runtime>(
    app: &tauri::App<R>,
) -> Result<TrayIcon<R>, Box<dyn std::error::Error>> {
    let handle = app.handle();

    let show_item = MenuItem::with_id(handle, TRAY_ID_SHOW, "Show DeskPilot", true, None::<&str>)?;
    let hide_item = MenuItem::with_id(handle, TRAY_ID_HIDE, "Hide DeskPilot", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(handle, TRAY_ID_QUIT, "Quit DeskPilot", true, None::<&str>)?;
    let menu = Menu::with_items(handle, &[&show_item, &hide_item, &quit_item])?;

    let icon = resolve_tray_icon(handle);

    let tray = TrayIconBuilder::with_id("deskpilot-tray")
        .tooltip(TRAY_TOOLTIP)
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            handle_tray_menu_action(app, event.id.as_ref());
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                let _ = show_main_window(app);
            }
        })
        .build(app)?;

    Ok(tray)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tray_id_constants() {
        assert_eq!(TRAY_ID_SHOW, "tray_show");
        assert_eq!(TRAY_ID_HIDE, "tray_hide");
        assert_eq!(TRAY_ID_QUIT, "tray_quit");
        assert_eq!(TRAY_TOOLTIP, "DeskPilot");
        assert_eq!(MAIN_WINDOW_LABEL, "main");
    }

    #[test]
    fn test_tray_action_parsing_and_unknown_handling() {
        assert_eq!(TrayAction::from_id(TRAY_ID_SHOW), Some(TrayAction::Show));
        assert_eq!(TrayAction::from_id(TRAY_ID_HIDE), Some(TrayAction::Hide));
        assert_eq!(TrayAction::from_id(TRAY_ID_QUIT), Some(TrayAction::Quit));

        assert_eq!(TrayAction::from_id("unknown"), None);
        assert_eq!(TrayAction::from_id(""), None);
        assert_eq!(TrayAction::from_id("tray_restart"), None);
    }

    #[test]
    fn test_menu_id_dispatch_mapping() {
        let valid_actions = [TRAY_ID_SHOW, TRAY_ID_HIDE, TRAY_ID_QUIT];
        for action in valid_actions {
            assert!(!action.is_empty());
            assert!(TrayAction::from_id(action).is_some());
        }
    }

    #[test]
    fn test_quitting_state_lifecycle() {
        let state = AppState::new("DeskPilot", "0.1.0");
        assert!(!state.is_quitting());

        state.set_quitting(true);
        assert!(state.is_quitting());

        state.set_quitting(false);
        assert!(!state.is_quitting());
    }

    #[test]
    fn test_close_requested_interception_policy() {
        let state = AppState::new("DeskPilot", "0.1.0");

        // Normal execution: window close MUST be prevented (hide to tray)
        assert!(should_prevent_close(&state));

        // Explicit quit: window close MUST NOT be prevented (allow process termination)
        state.set_quitting(true);
        assert!(!should_prevent_close(&state));

        // Reset: returns to normal hide-to-tray mode
        state.set_quitting(false);
        assert!(should_prevent_close(&state));
    }

    #[test]
    fn test_repeated_quitting_state_idempotence() {
        let state = AppState::new("DeskPilot", "0.1.0");

        // Repeated true
        state.set_quitting(true);
        state.set_quitting(true);
        assert!(state.is_quitting());
        assert!(!should_prevent_close(&state));

        // Repeated false
        state.set_quitting(false);
        state.set_quitting(false);
        assert!(!state.is_quitting());
        assert!(should_prevent_close(&state));
    }

    #[test]
    fn test_tray_icon_dimensions_and_bytes() {
        let img = tauri::image::Image::new_owned(vec![255u8; 16 * 16 * 4], 16, 16);
        assert_eq!(img.width(), 16);
        assert_eq!(img.height(), 16);
        assert_eq!(img.rgba().len(), 16 * 16 * 4);
    }
}
