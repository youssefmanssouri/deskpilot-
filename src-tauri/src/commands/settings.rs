use crate::commands::launcher::{register_launcher_shortcut, unregister_launcher_shortcut};
use crate::core::LaunchHistoryStore;
use crate::core::launcher::LauncherShortcutState;
use crate::core::settings::{
    SettingsStore, StartupMutationError, StartupRegistry, StartupState, ThemePreference,
    disable_startup_guarded, enable_startup_guarded, normalize_shortcut, shortcut_label,
};
use crate::error::IpcErrorDto;
use crate::platform;
use serde::Serialize;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager, Runtime};

/// Event broadcast to all windows after a setting changes (e.g. the launcher re-applies the theme).
pub const SETTINGS_CHANGED_EVENT: &str = "deskpilot://settings-changed";

/// Whether the glass effect is possible on this PC and whether it is on in this session.
/// Set once in `setup`; changing the setting only takes effect after a restart.
#[derive(Debug, Default)]
pub struct GlassState {
    pub supported: AtomicBool,
    pub active: AtomicBool,
}

/// Sanitized startup registration state exposed over Tauri IPC.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StartupStatusState {
    Disabled,
    Enabled,
    Stale,
    OtherInstall,
    Unrecognized,
    Unavailable,
}

/// Detailed startup registration status returned inside [`SettingsDto`].
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StartupStatusDto {
    pub state: StartupStatusState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
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
    /// Backward-compatible boolean view of startup status; `None` when stale,
    /// owned by another install, unrecognized, or unavailable.
    pub start_with_windows: Option<bool>,
    /// Detailed sanitized startup status.
    pub startup_status: StartupStatusDto,
    /// The saved preference.
    pub glass_effect: bool,
    /// True on Windows 11.
    pub glass_supported: bool,
    /// True if the glass effect is on in this session.
    pub glass_active: bool,
}

fn map_startup_status(
    result: Result<StartupState, StartupMutationError>,
) -> (Option<bool>, StartupStatusDto) {
    match result {
        Ok(StartupState::Disabled) => (
            Some(false),
            StartupStatusDto {
                state: StartupStatusState::Disabled,
                error_code: None,
            },
        ),
        Ok(StartupState::Enabled) => (
            Some(true),
            StartupStatusDto {
                state: StartupStatusState::Enabled,
                error_code: None,
            },
        ),
        Ok(StartupState::Stale) => (
            None,
            StartupStatusDto {
                state: StartupStatusState::Stale,
                error_code: None,
            },
        ),
        Ok(StartupState::OtherInstall) => (
            None,
            StartupStatusDto {
                state: StartupStatusState::OtherInstall,
                error_code: None,
            },
        ),
        Ok(StartupState::Unrecognized) => (
            None,
            StartupStatusDto {
                state: StartupStatusState::Unrecognized,
                error_code: None,
            },
        ),
        Err(err) => {
            let error_code = match err {
                StartupMutationError::PermissionDenied => "PERMISSION_DENIED",
                StartupMutationError::InvalidState(_) | StartupMutationError::Conflict(_) => {
                    "INVALID_STARTUP_STATE"
                }
                StartupMutationError::IoFailure(_) => "OPERATION_FAILED",
            };
            (
                None,
                StartupStatusDto {
                    state: StartupStatusState::Unavailable,
                    error_code: Some(error_code.to_string()),
                },
            )
        }
    }
}

fn map_startup_mutation_error(err: StartupMutationError, enabled: bool) -> IpcErrorDto {
    match err {
        StartupMutationError::PermissionDenied => IpcErrorDto {
            code: "PERMISSION_DENIED".to_string(),
            message: if enabled {
                "Permission denied while adding DeskPilot to the Windows startup list."
            } else {
                "Permission denied while removing DeskPilot from the Windows startup list."
            }
            .to_string(),
            details: None,
        },
        StartupMutationError::Conflict(StartupState::Stale) => IpcErrorDto {
            code: "STARTUP_CONFLICT".to_string(),
            message: "A stale DeskPilot startup entry already exists and was not overwritten or deleted."
                .to_string(),
            details: None,
        },
        StartupMutationError::Conflict(StartupState::OtherInstall) => IpcErrorDto {
            code: "STARTUP_CONFLICT".to_string(),
            message: "Another DeskPilot installation owns the Windows startup entry and was not modified."
                .to_string(),
            details: None,
        },
        StartupMutationError::Conflict(StartupState::Unrecognized) => IpcErrorDto {
            code: "INVALID_STARTUP_STATE".to_string(),
            message: "An unrecognized Windows startup entry already exists and was not modified."
                .to_string(),
            details: None,
        },
        StartupMutationError::Conflict(_) | StartupMutationError::InvalidState(_) => IpcErrorDto {
            code: "INVALID_STARTUP_STATE".to_string(),
            message: "The current Windows startup state does not permit that operation."
                .to_string(),
            details: None,
        },
        StartupMutationError::IoFailure(_) => IpcErrorDto {
            code: "OPERATION_FAILED".to_string(),
            message: if enabled {
                "DeskPilot could not be added to the Windows startup list."
            } else {
                "DeskPilot could not be removed from the Windows startup list."
            }
            .to_string(),
            details: None,
        },
    }
}

#[cfg(test)]
fn inspect_startup_with(
    registry: &dyn StartupRegistry,
    current_exe: Option<&Path>,
    expected_arg: &str,
    path_exists: &dyn Fn(&Path) -> bool,
) -> (Option<bool>, StartupStatusDto) {
    use crate::core::settings::inspect_startup_state;
    let result = match current_exe {
        Some(exe) => inspect_startup_state(registry, exe, expected_arg, path_exists),
        None => Err(StartupMutationError::InvalidState(
            "Current executable path is unavailable.",
        )),
    };
    map_startup_status(result)
}

fn apply_start_with_windows(
    registry: &dyn StartupRegistry,
    current_exe: Option<&Path>,
    expected_arg: &str,
    path_exists: &dyn Fn(&Path) -> bool,
    enabled: bool,
) -> Result<(), StartupMutationError> {
    let exe = current_exe.ok_or(StartupMutationError::InvalidState(
        "Current executable path is unavailable.",
    ))?;
    if enabled {
        enable_startup_guarded(registry, exe, expected_arg, path_exists)
    } else {
        disable_startup_guarded(registry, exe, expected_arg, path_exists)
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
    #[cfg(target_os = "windows")]
    let (start_with_windows, startup_status) =
        map_startup_status(platform::inspect_start_with_windows_state());
    #[cfg(not(target_os = "windows"))]
    let (start_with_windows, startup_status) = map_startup_status(Err(
        StartupMutationError::IoFailure(std::io::ErrorKind::Unsupported),
    ));
    Ok(SettingsDto {
        theme: settings.theme,
        history_enabled: settings.history_enabled,
        launcher_shortcut_label: shortcut_label(&settings.launcher_shortcut),
        launcher_shortcut: settings.launcher_shortcut,
        shortcut_registered: shortcut.registered,
        shortcut_message: shortcut.message,
        start_with_windows,
        startup_status,
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

/// Adds or removes DeskPilot's entry in the current user's Windows startup list
/// (HKCU Run key, never the machine-wide key).
/// When Windows starts DeskPilot, it opens in the tray instead of showing the window.
#[tauri::command]
pub fn set_start_with_windows<R: Runtime>(
    app: AppHandle<R>,
    enabled: bool,
) -> Result<SettingsDto, IpcErrorDto> {
    #[cfg(target_os = "windows")]
    {
        let exe = std::env::current_exe()
            .map_err(|err| map_startup_mutation_error(StartupMutationError::from(err), enabled))?;
        apply_start_with_windows(
            &platform::WindowsRunKeyStore::current_user_run(),
            Some(&exe),
            crate::AUTOSTART_ARG,
            &|p: &Path| p.is_file(),
            enabled,
        )
        .map_err(|err| map_startup_mutation_error(err, enabled))?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = enabled;
        return Err(map_startup_mutation_error(
            StartupMutationError::IoFailure(std::io::ErrorKind::Unsupported),
            enabled,
        ));
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::settings::StartupRegistryValue;
    use serde_json::{Value, json};
    use std::io;
    use std::sync::Mutex;

    #[derive(Debug, Default)]
    struct MockRegistry {
        value: Mutex<Option<StartupRegistryValue>>,
        read_error: Option<io::ErrorKind>,
        write_error: Option<io::ErrorKind>,
        delete_error: Option<io::ErrorKind>,
    }

    impl MockRegistry {
        fn empty() -> Self {
            Self::default()
        }

        fn with_str(raw: &str) -> Self {
            Self {
                value: Mutex::new(Some(StartupRegistryValue::String(raw.to_string()))),
                ..Default::default()
            }
        }

        fn with_read_error(kind: io::ErrorKind) -> Self {
            Self {
                read_error: Some(kind),
                ..Default::default()
            }
        }

        fn current_value(&self) -> Option<StartupRegistryValue> {
            self.value.lock().unwrap().clone()
        }
    }

    impl StartupRegistry for MockRegistry {
        fn read_value(&self) -> io::Result<Option<StartupRegistryValue>> {
            if let Some(kind) = self.read_error {
                return Err(io::Error::new(kind, "mock read error"));
            }
            Ok(self.value.lock().unwrap().clone())
        }

        fn write_value(&self, command: &str) -> io::Result<()> {
            if let Some(kind) = self.write_error {
                return Err(io::Error::new(kind, "mock write error"));
            }
            *self.value.lock().unwrap() = Some(StartupRegistryValue::String(command.to_string()));
            Ok(())
        }

        fn delete_value(&self) -> io::Result<()> {
            if let Some(kind) = self.delete_error {
                return Err(io::Error::new(kind, "mock delete error"));
            }
            *self.value.lock().unwrap() = None;
            Ok(())
        }
    }

    fn exe_path() -> &'static Path {
        Path::new(r"C:\Program Files\DeskPilot\deskpilot.exe")
    }

    fn dto_with_startup(
        start_with_windows: Option<bool>,
        startup_status: StartupStatusDto,
    ) -> Value {
        let dto = SettingsDto {
            theme: ThemePreference::System,
            history_enabled: true,
            launcher_shortcut: "Ctrl+Alt+Space".to_string(),
            launcher_shortcut_label: "Ctrl+Alt+Space".to_string(),
            shortcut_registered: true,
            shortcut_message: None,
            start_with_windows,
            startup_status,
            glass_effect: false,
            glass_supported: true,
            glass_active: false,
        };
        serde_json::to_value(&dto).unwrap()
    }

    #[test]
    fn serializes_disabled_and_enabled_states_with_backward_compatible_bool() {
        let disabled_reg = MockRegistry::empty();
        let (compat, status) =
            inspect_startup_with(&disabled_reg, Some(exe_path()), "--autostart", &|_| false);
        assert_eq!(compat, Some(false));
        assert_eq!(
            serde_json::to_value(&status).unwrap(),
            json!({ "state": "disabled" })
        );
        let full_disabled = dto_with_startup(compat, status);
        assert_eq!(full_disabled["startWithWindows"], json!(false));
        assert_eq!(
            full_disabled["startupStatus"],
            json!({ "state": "disabled" })
        );

        let enabled_reg =
            MockRegistry::with_str(r#""C:\Program Files\DeskPilot\deskpilot.exe" --autostart"#);
        let (compat, status) =
            inspect_startup_with(&enabled_reg, Some(exe_path()), "--autostart", &|_| false);
        assert_eq!(compat, Some(true));
        assert_eq!(
            serde_json::to_value(&status).unwrap(),
            json!({ "state": "enabled" })
        );
        let full_enabled = dto_with_startup(compat, status);
        assert_eq!(full_enabled["startWithWindows"], json!(true));
        assert_eq!(full_enabled["startupStatus"], json!({ "state": "enabled" }));
    }

    #[test]
    fn serializes_stale_other_install_unrecognized_and_unavailable_with_null_compat() {
        let stale_reg =
            MockRegistry::with_str(r#""C:\LegacyInstall\DeskPilot\deskpilot.exe" --autostart"#);
        let (compat, status) =
            inspect_startup_with(&stale_reg, Some(exe_path()), "--autostart", &|_| false);
        assert_eq!(compat, None);
        assert_eq!(
            serde_json::to_value(&status).unwrap(),
            json!({ "state": "stale" })
        );

        let other_reg =
            MockRegistry::with_str(r#""D:\Alternate\DeskPilot\deskpilot.exe" --autostart"#);
        let (compat, status) =
            inspect_startup_with(&other_reg, Some(exe_path()), "--autostart", &|_| true);
        assert_eq!(compat, None);
        assert_eq!(
            serde_json::to_value(&status).unwrap(),
            json!({ "state": "otherInstall" })
        );

        let foreign_reg = MockRegistry::with_str(r#""C:\ThirdParty\other.exe" --secret-flag"#);
        let (compat, status) =
            inspect_startup_with(&foreign_reg, Some(exe_path()), "--autostart", &|_| true);
        assert_eq!(compat, None);
        assert_eq!(
            serde_json::to_value(&status).unwrap(),
            json!({ "state": "unrecognized" })
        );

        let denied_reg = MockRegistry::with_read_error(io::ErrorKind::PermissionDenied);
        let (compat, status) =
            inspect_startup_with(&denied_reg, Some(exe_path()), "--autostart", &|_| false);
        assert_eq!(compat, None);
        assert_eq!(
            serde_json::to_value(&status).unwrap(),
            json!({ "state": "unavailable", "errorCode": "PERMISSION_DENIED" })
        );

        let io_err_reg = MockRegistry::with_read_error(io::ErrorKind::Other);
        let (compat, status) =
            inspect_startup_with(&io_err_reg, Some(exe_path()), "--autostart", &|_| false);
        assert_eq!(compat, None);
        assert_eq!(
            serde_json::to_value(&status).unwrap(),
            json!({ "state": "unavailable", "errorCode": "OPERATION_FAILED" })
        );
    }

    #[test]
    fn maps_mutation_errors_precisely_without_leaking_paths_or_values() {
        // 1. Permission denied (write and delete)
        let write_denied = MockRegistry {
            write_error: Some(io::ErrorKind::PermissionDenied),
            ..Default::default()
        };
        let err = apply_start_with_windows(
            &write_denied,
            Some(exe_path()),
            "--autostart",
            &|_| false,
            true,
        )
        .unwrap_err();
        let dto = map_startup_mutation_error(err, true);
        assert_eq!(dto.code, "PERMISSION_DENIED");
        assert!(dto.details.is_none());

        // 2. Conflict on Stale / OtherInstall (preserves value untouched)
        let stale_val = r#""C:\LegacyInstall\DeskPilot\deskpilot.exe" --autostart"#;
        let stale_reg = MockRegistry::with_str(stale_val);
        for enabled in [true, false] {
            let err = apply_start_with_windows(
                &stale_reg,
                Some(exe_path()),
                "--autostart",
                &|_| false,
                enabled,
            )
            .unwrap_err();
            let dto = map_startup_mutation_error(err, enabled);
            assert_eq!(dto.code, "STARTUP_CONFLICT");
            assert!(!dto.message.contains("LegacyInstall"));
            assert!(!dto.message.contains("CurrentVersion"));
        }
        assert_eq!(
            stale_reg.current_value(),
            Some(StartupRegistryValue::String(stale_val.to_string()))
        );

        let other_val = r#""D:\Alternate\DeskPilot\deskpilot.exe" --autostart"#;
        let other_reg = MockRegistry::with_str(other_val);
        let err =
            apply_start_with_windows(&other_reg, Some(exe_path()), "--autostart", &|_| true, true)
                .unwrap_err();
        let dto = map_startup_mutation_error(err, true);
        assert_eq!(dto.code, "STARTUP_CONFLICT");
        assert!(!dto.message.contains("Alternate"));

        // 3. Invalid / Unrecognized state (foreign entry & missing current_exe)
        let foreign_val = r#""C:\SecretTool\helper.exe" --token=abc"#;
        let foreign_reg = MockRegistry::with_str(foreign_val);
        for enabled in [true, false] {
            let err = apply_start_with_windows(
                &foreign_reg,
                Some(exe_path()),
                "--autostart",
                &|_| true,
                enabled,
            )
            .unwrap_err();
            let dto = map_startup_mutation_error(err, enabled);
            assert_eq!(dto.code, "INVALID_STARTUP_STATE");
            assert!(!dto.message.contains("SecretTool"));
            assert!(!dto.message.contains("helper.exe"));
            assert!(!dto.message.contains("token=abc"));
        }

        let missing_exe_err = apply_start_with_windows(
            &MockRegistry::empty(),
            None,
            "--autostart",
            &|_| false,
            true,
        )
        .unwrap_err();
        let dto = map_startup_mutation_error(missing_exe_err, true);
        assert_eq!(dto.code, "INVALID_STARTUP_STATE");

        // 4. Unexpected OS / I/O failure
        let io_reg = MockRegistry {
            write_error: Some(io::ErrorKind::Other),
            ..Default::default()
        };
        let err =
            apply_start_with_windows(&io_reg, Some(exe_path()), "--autostart", &|_| false, true)
                .unwrap_err();
        let dto = map_startup_mutation_error(err, true);
        assert_eq!(dto.code, "OPERATION_FAILED");
    }
}
