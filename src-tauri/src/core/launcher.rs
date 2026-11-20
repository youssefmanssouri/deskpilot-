//! Quick launcher: constants, navigation validation and global-shortcut status.
//!
//! No Tauri or Windows dependencies, so the logic is unit-testable on any platform.
//! Window handling lives in `commands::launcher`.

use crate::error::DomainError;
use crate::security::InputGuard;
use serde::Serialize;
use std::sync::Mutex;

/// Label of the launcher window declared in `tauri.conf.json`.
pub const LAUNCHER_WINDOW_LABEL: &str = "launcher";

/// Default global shortcut, in the format `tauri-plugin-global-shortcut` parses.
pub const DEFAULT_LAUNCHER_SHORTCUT: &str = "ctrl+alt+space";

/// Human-readable form of the default shortcut, shown in the UI.
pub const DEFAULT_LAUNCHER_SHORTCUT_LABEL: &str = "Ctrl+Alt+Space";

/// Event emitted to the main window to switch view (and optionally select an app).
pub const NAVIGATE_EVENT: &str = "deskpilot://navigate";

/// Event emitted to the launcher window each time it is shown.
pub const LAUNCHER_SHOWN_EVENT: &str = "deskpilot://launcher-shown";

/// Views the main window accepts. Mirrors the frontend's `CatalogView`.
pub const NAVIGABLE_VIEWS: [&str; 4] = ["home", "applications", "processes", "system"];

/// Validated request for the main window to change view.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NavigateRequest {
    pub view: String,
    pub application_id: Option<String>,
}

/// Validates a navigation request coming from the (untrusted) launcher page.
/// Only known views are accepted; an application id must have the catalog id format
/// and is only allowed together with the "applications" view.
pub fn validate_navigate_request(
    view: &str,
    application_id: Option<&str>,
) -> Result<NavigateRequest, DomainError> {
    if !NAVIGABLE_VIEWS.contains(&view) {
        return Err(DomainError::InvalidInput(format!(
            "Unknown view. Expected one of: {}.",
            NAVIGABLE_VIEWS.join(", ")
        )));
    }
    if let Some(id) = application_id {
        if view != "applications" {
            return Err(DomainError::InvalidInput(
                "An application can only be selected in the applications view.".to_string(),
            ));
        }
        InputGuard::validate_application_id(id)?;
    }
    Ok(NavigateRequest {
        view: view.to_string(),
        application_id: application_id.map(str::to_string),
    })
}

/// Whether the global shortcut could be registered, for display in the UI.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutStatus {
    /// Human-readable shortcut, e.g. "Ctrl+Alt+Space".
    pub accelerator: String,
    pub registered: bool,
    /// Plain-language explanation when registration failed.
    pub message: Option<String>,
}

/// Thread-safe holder for the shortcut status, managed as Tauri state.
#[derive(Debug)]
pub struct LauncherShortcutState {
    status: Mutex<ShortcutStatus>,
}

impl Default for LauncherShortcutState {
    fn default() -> Self {
        Self::new()
    }
}

impl LauncherShortcutState {
    /// Starts as "not registered" until setup reports the outcome.
    pub fn new() -> Self {
        Self {
            status: Mutex::new(ShortcutStatus {
                accelerator: DEFAULT_LAUNCHER_SHORTCUT_LABEL.to_string(),
                registered: false,
                message: Some("The shortcut has not been registered yet.".to_string()),
            }),
        }
    }

    pub fn set_registered(&self) {
        if let Ok(mut s) = self.status.lock() {
            s.registered = true;
            s.message = None;
        }
    }

    /// Records a registration failure. The raw plugin error is not shown to users;
    /// the message explains the likely cause and what to do.
    pub fn set_failed(&self) {
        if let Ok(mut s) = self.status.lock() {
            s.registered = false;
            s.message = Some(format!(
                "{} is already used by another app, so the quick launcher can't open with it. \
                 Close the other app or change its shortcut, then restart DeskPilot.",
                s.accelerator
            ));
        }
    }

    pub fn status(&self) -> Result<ShortcutStatus, DomainError> {
        self.status
            .lock()
            .map(|s| s.clone())
            .map_err(|_| DomainError::InternalError("Shortcut status lock poisoned".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "app_0123456789abcdef0123456789abcdef";

    #[test]
    fn accepts_known_views_without_an_app() {
        for view in NAVIGABLE_VIEWS {
            let req = validate_navigate_request(view, None).unwrap();
            assert_eq!(req.view, view);
            assert_eq!(req.application_id, None);
        }
    }

    #[test]
    fn accepts_an_app_only_in_the_applications_view() {
        let req = validate_navigate_request("applications", Some(ID)).unwrap();
        assert_eq!(req.application_id.as_deref(), Some(ID));
        assert!(validate_navigate_request("system", Some(ID)).is_err());
    }

    #[test]
    fn rejects_unknown_views_and_malformed_ids() {
        assert!(validate_navigate_request("settings", None).is_err());
        assert!(validate_navigate_request("", None).is_err());
        assert!(validate_navigate_request("applications", Some("../../etc/passwd")).is_err());
        assert!(validate_navigate_request("applications", Some("APP_0123")).is_err());
    }

    #[test]
    fn navigate_request_serializes_in_camel_case() {
        let json =
            serde_json::to_string(&validate_navigate_request("applications", Some(ID)).unwrap())
                .unwrap();
        assert_eq!(
            json,
            format!(r#"{{"view":"applications","applicationId":"{ID}"}}"#)
        );
    }

    #[test]
    fn shortcut_status_transitions() {
        let state = LauncherShortcutState::new();
        let initial = state.status().unwrap();
        assert!(!initial.registered);
        assert_eq!(initial.accelerator, "Ctrl+Alt+Space");

        state.set_registered();
        let ok = state.status().unwrap();
        assert!(ok.registered);
        assert_eq!(ok.message, None);

        state.set_failed();
        let failed = state.status().unwrap();
        assert!(!failed.registered);
        assert!(
            failed
                .message
                .unwrap()
                .contains("Ctrl+Alt+Space is already used")
        );
    }
}
