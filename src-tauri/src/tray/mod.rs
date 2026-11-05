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
/// Returns `true` if system tray is active and no explicit quit was requested (hide to tray).
/// Returns `false` if the tray is unavailable (fallback windowed mode) or if an explicit quit
/// has been triggered (allow termination).
pub fn should_prevent_close(state: &AppState) -> bool {
    state.is_tray_available() && !state.is_quitting()
}

/// Intercepts window close requests to implement predictable hide-to-tray behavior.
///
/// If system tray is active and the application is not actively quitting, closing the window
/// hides it and cancels the close request.
/// If system tray is unavailable (fallback windowed mode) or an explicit quit was initiated,
/// the close request is allowed to proceed without interference, permitting clean window
/// destruction and application shutdown.
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

/// Handles fallback transition to windowed mode when tray setup has failed.
///
/// Records `tray_available = false` on `AppState`, logs the diagnostic warning,
/// and executes the window activation callback.
///
/// Returns `Ok(false)` if window activation succeeds, allowing startup to continue
/// in standard windowed mode.
/// Returns `Err` if window activation fails, propagating a fatal startup error that
/// describes both the tray initialization failure and the fallback window activation failure.
pub fn handle_tray_init_fallback<F, E>(
    state: &AppState,
    tray_err: &dyn std::error::Error,
    activate_window_fn: F,
) -> Result<bool, Box<dyn std::error::Error>>
where
    F: FnOnce() -> Result<(), E>,
    E: std::fmt::Display,
{
    eprintln!(
        "WARN: Windows system tray initialization failed: {tray_err}. Falling back to standard windowed mode."
    );
    state.set_tray_available(false);

    activate_window_fn().map_err(|win_err| {
        format!(
            "Failed to initialize system tray ({tray_err}) and failed to activate fallback main window: {win_err}"
        )
    })?;

    Ok(false)
}

/// Initializes the system tray if supported by the host environment, or gracefully
/// falls back to standard windowed mode if tray creation fails.
///
/// If tray initialization succeeds, records `tray_available = true` on `AppState`
/// and returns `Ok(true)`.
/// If tray initialization fails (e.g. headless CI, missing shell notification area,
/// or platform limitations), logs a concise warning diagnostic to stderr, records
/// `tray_available = false` on `AppState`, and attempts to restore, unminimize, show,
/// and focus the fallback main window.
///
/// If fallback main window activation succeeds, returns `Ok(false)` to permit standard
/// windowed execution.
/// If fallback main window activation fails, returns a fatal `Err` describing both
/// the tray initialization failure and the window activation failure, cleanly aborting
/// startup rather than stranding an inaccessible background process.
pub fn init_tray_lifecycle<R: Runtime>(
    app: &tauri::App<R>,
) -> Result<bool, Box<dyn std::error::Error>> {
    let state = app.state::<AppState>();
    match setup_tray(app) {
        Ok(_tray) => {
            state.set_tray_available(true);
            Ok(true)
        }
        Err(err) => {
            handle_tray_init_fallback(&state, err.as_ref(), || show_main_window(app.handle()))
        }
    }
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

        // Uninitialized / fallback mode: close MUST NOT be prevented (allow standard window close)
        assert!(!should_prevent_close(&state));

        // Tray mode activated: window close MUST be prevented (hide to tray)
        state.set_tray_available(true);
        assert!(should_prevent_close(&state));

        // Explicit quit in tray mode: window close MUST NOT be prevented (allow process termination)
        state.set_quitting(true);
        assert!(!should_prevent_close(&state));

        // Quitting reset in tray mode: returns to normal hide-to-tray mode
        state.set_quitting(false);
        assert!(should_prevent_close(&state));

        // Tray unavailable (fallback mode): window close MUST NOT be prevented even when not quitting
        state.set_tray_available(false);
        assert!(!should_prevent_close(&state));
    }

    #[test]
    fn test_repeated_quitting_state_idempotence() {
        let state = AppState::new("DeskPilot", "0.1.0");
        state.set_tray_available(true);

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
    fn test_tray_availability_state_idempotence() {
        let state = AppState::new("DeskPilot", "0.1.0");
        assert!(!state.is_tray_available());

        // Repeated true
        state.set_tray_available(true);
        state.set_tray_available(true);
        assert!(state.is_tray_available());
        assert!(should_prevent_close(&state));

        // Repeated false
        state.set_tray_available(false);
        state.set_tray_available(false);
        assert!(!state.is_tray_available());
        assert!(!should_prevent_close(&state));
    }

    #[test]
    fn test_close_requested_interception_matrix() {
        let state = AppState::new("DeskPilot", "0.1.0");

        // Matrix case 1: tray unavailable, not quitting -> do NOT prevent close (fallback mode)
        state.set_tray_available(false);
        state.set_quitting(false);
        assert!(!should_prevent_close(&state));

        // Matrix case 2: tray unavailable, quitting -> do NOT prevent close (fallback mode exit)
        state.set_tray_available(false);
        state.set_quitting(true);
        assert!(!should_prevent_close(&state));

        // Matrix case 3: tray available, not quitting -> prevent close (hide to tray)
        state.set_tray_available(true);
        state.set_quitting(false);
        assert!(should_prevent_close(&state));

        // Matrix case 4: tray available, quitting -> do NOT prevent close (tray mode exit)
        state.set_tray_available(true);
        state.set_quitting(true);
        assert!(!should_prevent_close(&state));
    }

    #[test]
    fn test_tray_icon_dimensions_and_bytes() {
        let img = tauri::image::Image::new_owned(vec![255u8; 16 * 16 * 4], 16, 16);
        assert_eq!(img.width(), 16);
        assert_eq!(img.height(), 16);
        assert_eq!(img.rgba().len(), 16 * 16 * 4);
    }

    #[test]
    fn test_handle_tray_init_fallback_window_activation_success() {
        let state = AppState::new("DeskPilot", "0.1.0");
        state.set_tray_available(true); // simulate pre-existing true state

        let tray_err = std::io::Error::new(std::io::ErrorKind::Other, "Shell_NotifyIconW failed");
        let result = handle_tray_init_fallback(&state, &tray_err, || Ok::<(), String>(()));

        // Successful activation returns Ok(false) (fallback mode active)
        assert_eq!(result.unwrap(), false);
        // State must be set to false
        assert!(!state.is_tray_available());
        // Close policy must allow window close
        assert!(!should_prevent_close(&state));
    }

    #[test]
    fn test_handle_tray_init_fallback_window_activation_failure_propagates_both_errors() {
        let state = AppState::new("DeskPilot", "0.1.0");
        state.set_tray_available(true);

        let tray_err =
            std::io::Error::new(std::io::ErrorKind::NotFound, "Tray service unavailable");
        let result = handle_tray_init_fallback(&state, &tray_err, || {
            Err::<(), String>("Main window 'main' not found".to_string())
        });

        // Must propagate fatal error
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();

        // Must identify both the tray initialization failure and the window activation failure
        assert!(err_msg.contains("Failed to initialize system tray (Tray service unavailable)"));
        assert!(
            err_msg
                .contains("failed to activate fallback main window: Main window 'main' not found")
        );

        // State must remain false
        assert!(!state.is_tray_available());
    }

    #[test]
    fn test_handle_tray_init_fallback_idempotence_and_quitting_interaction() {
        let state = AppState::new("DeskPilot", "0.1.0");
        state.set_quitting(true);

        let tray_err = std::io::Error::new(std::io::ErrorKind::Other, "OS error 1000");
        let result = handle_tray_init_fallback(&state, &tray_err, || Ok::<(), &str>(()));

        assert_eq!(result.unwrap(), false);
        assert!(!state.is_tray_available());
        assert!(state.is_quitting());
        assert!(!should_prevent_close(&state));
    }
}
