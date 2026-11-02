use crate::core::ApplicationLauncherService;
use crate::error::IpcErrorDto;
use crate::models::{ApplicationDto, DiscoveryResultDto, LaunchResultDto};
use crate::security::InputGuard;
use crate::state::AppState;
use tauri::State;

/// Thin Tauri command controller for discovering installed Windows applications.
///
/// Dispatches serialized discovery through `AppState`, atomically updating the
/// in-memory catalog with internal domain models, and maps the result into a safe
/// `DiscoveryResultDto` where sensitive machine paths are omitted.
#[tauri::command]
pub fn discover_applications(
    state: State<'_, AppState>,
) -> Result<DiscoveryResultDto, IpcErrorDto> {
    let result = state
        .execute_serialized_discovery()
        .map_err(IpcErrorDto::from)?;

    let app_dtos: Vec<ApplicationDto> = result
        .applications
        .into_iter()
        .map(ApplicationDto::from)
        .collect();

    let total = app_dtos.len();

    Ok(DiscoveryResultDto {
        applications: app_dtos,
        scannedSources: result.scanned_sources,
        partialFailures: result.partial_failures,
        totalDiscovered: total,
    })
}

/// Thin Tauri command controller for launching an application by its catalog ID.
///
/// Dispatches execution to `ApplicationLauncherService::launch`, which validates
/// the identifier against the input guard and the in-memory catalog, enforces strict
/// executable path policies (no transient/temp dirs, canonical .exe check, no UNC/device
/// syntax), and spawns the process natively with zero caller arguments and no shell.
///
/// Maps the result into a safe `LaunchResultDto` containing only `applicationId` and `processId`.
#[tauri::command]
pub fn launch_application(
    state: State<'_, AppState>,
    application_id: String,
) -> Result<LaunchResultDto, IpcErrorDto> {
    launch_application_impl(&state, &application_id)
}

/// Internal implementation of `launch_application` separated from `State` extraction
/// to permit direct, deterministic unit testing without requiring Tauri runtime mocking.
pub fn launch_application_impl(
    state: &AppState,
    application_id: &str,
) -> Result<LaunchResultDto, IpcErrorDto> {
    InputGuard::validate_application_id(application_id).map_err(IpcErrorDto::from)?;
    let result =
        ApplicationLauncherService::launch(state, application_id).map_err(IpcErrorDto::from)?;
    Ok(LaunchResultDto::from(result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{AvailabilityState, DiscoveredApplication, DiscoverySource};
    use std::path::PathBuf;

    fn get_test_system_executable() -> PathBuf {
        let candidates = [
            r"C:\Windows\System32\whoami.exe",
            r"C:\Windows\System32\where.exe",
        ];
        for candidate in candidates {
            let p = PathBuf::from(candidate);
            if p.is_file() {
                return p;
            }
        }
        panic!("Standard Windows test executable not found in System32");
    }

    fn create_test_app(
        id: &str,
        target: Option<String>,
        availability: AvailabilityState,
    ) -> DiscoveredApplication {
        DiscoveredApplication {
            id: id.to_string(),
            display_name: "Test App".to_string(),
            publisher: Some("Test Publisher".to_string()),
            version: Some("1.0.0".to_string()),
            sources: vec![DiscoverySource::TestFixture],
            install_location: None,
            target_path: target,
            availability,
            discovered_at: 1000,
        }
    }

    #[test]
    fn test_launch_command_valid_application_reaches_launcher() {
        let state = AppState::new("DeskPilotTest", "1.0.0");
        let valid_id = "app_1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d";
        let target = get_test_system_executable().to_string_lossy().to_string();
        let app = create_test_app(valid_id, Some(target), AvailabilityState::Available);
        state.update_catalog(vec![app]).unwrap();

        let result = launch_application_impl(&state, valid_id);
        assert!(
            result.is_ok(),
            "Expected launch to succeed for valid System32 executable"
        );
        let dto = result.unwrap();
        assert_eq!(dto.application_id, valid_id);
        assert!(dto.process_id > 0);

        // Verify JSON serialization isolation
        let serialized = serde_json::to_string(&dto).unwrap();
        assert!(serialized.contains(r#""applicationId":"app_1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d""#));
        assert!(serialized.contains(r#""processId":"#));
        assert!(!serialized.contains("path"));
        assert!(!serialized.contains("executable"));
        assert!(!serialized.contains("args"));
        assert!(!serialized.contains("cwd"));
        assert!(!serialized.contains("env"));
    }

    #[test]
    fn test_launch_command_invalid_application_id_rejected() {
        let state = AppState::new("DeskPilotTest", "1.0.0");

        let invalid_ids = [
            "",
            "not_an_app_id",
            "app_short",
            "app_1234567890abcdef1234567890abcdef_too_long",
            "app_1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6Z", // uppercase / non-hex
            "app_1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6-",
            "../../etc/passwd",
            r"C:\Windows\System32\calc.exe",
        ];

        for invalid_id in invalid_ids {
            let res = launch_application_impl(&state, invalid_id);
            assert!(res.is_err(), "Expected rejection for ID: {}", invalid_id);
            let err = res.unwrap_err();
            assert_eq!(err.code, "INVALID_INPUT");
            assert!(
                !err.message.contains(r"C:\"),
                "Error must not leak filesystem paths"
            );
        }
    }

    #[test]
    fn test_launch_command_missing_catalog_application_rejected() {
        let state = AppState::new("DeskPilotTest", "1.0.0");
        let non_existent_id = "app_00000000000000000000000000000000";

        let res = launch_application_impl(&state, non_existent_id);
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert_eq!(err.code, "NOT_FOUND");
        assert!(err.message.contains("not found"));
        assert!(
            !err.message.contains(r"C:\"),
            "Error must not leak filesystem paths"
        );
    }

    #[test]
    fn test_launch_command_error_privacy_no_filesystem_leakage() {
        let state = AppState::new("DeskPilotTest", "1.0.0");
        let app_id = "app_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        // Target in Temp folder fails transient directory policy
        let mut temp_target = std::env::temp_dir();
        temp_target.push("untrusted_deskpilot_test.exe");
        std::fs::write(&temp_target, b"MZ").unwrap();
        let target_str = temp_target.to_string_lossy().to_string();
        let app = create_test_app(app_id, Some(target_str), AvailabilityState::Available);
        state.update_catalog(vec![app]).unwrap();

        let res = launch_application_impl(&state, app_id);
        let _ = std::fs::remove_file(&temp_target);
        assert!(res.is_err());
        let err = res.unwrap_err();
        // Policy rejection for temp directory is PermissionDenied
        assert_eq!(err.code, "PERMISSION_DENIED");

        let serialized = serde_json::to_string(&err).unwrap();
        // Ensure no path, username, or raw error leak across IPC boundary
        assert!(
            !serialized.contains(r"C:\"),
            "Serialized error must not contain paths: {}",
            serialized
        );
        assert!(
            !serialized.contains("untrusted_executable.exe"),
            "Serialized error must not contain file names: {}",
            serialized
        );
        assert!(
            !serialized.contains("os error"),
            "Serialized error must not leak raw OS error numbers: {}",
            serialized
        );
    }

    #[test]
    fn test_launch_result_dto_serialization_field_isolation() {
        let dto = LaunchResultDto {
            application_id: "app_1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d".to_string(),
            process_id: 12345,
        };

        let json = serde_json::to_value(&dto).unwrap();
        let obj = json.as_object().unwrap();

        // Exactly two fields: applicationId and processId
        assert_eq!(obj.len(), 2);
        assert_eq!(
            obj.get("applicationId").unwrap().as_str().unwrap(),
            "app_1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d"
        );
        assert_eq!(obj.get("processId").unwrap().as_u64().unwrap(), 12345);

        assert!(!obj.contains_key("path"));
        assert!(!obj.contains_key("executablePath"));
        assert!(!obj.contains_key("args"));
        assert!(!obj.contains_key("cwd"));
        assert!(!obj.contains_key("env"));
        assert!(!obj.contains_key("shell"));
    }

    #[test]
    fn test_launch_result_dto_camel_case_json_exact() {
        let dto = LaunchResultDto {
            application_id: "app_1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d".to_string(),
            process_id: 42,
        };

        let serialized = serde_json::to_string(&dto).unwrap();
        assert_eq!(
            serialized,
            r#"{"applicationId":"app_1a2b3c4d5e6f7a8b9c0d1e2f3a4b5c6d","processId":42}"#
        );
    }
}
