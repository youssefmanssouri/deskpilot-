use crate::core::SystemService;
use crate::error::{DomainError, IpcErrorDto};
use crate::models::SystemMetricsDto;
use crate::state::AppState;
use tauri::Manager;

/// Thin Tauri command controller for querying System Intelligence metrics.
///
/// Dispatches execution to a dedicated blocking worker thread pool via `tauri::async_runtime::spawn_blocking`,
/// ensuring synchronous Win32 calls and coordinator sleeps never block Tokio async workers or the application UI.
///
/// Accepts no user-controlled arguments, shell strings, or process identifiers.
#[tauri::command]
pub async fn get_system_metrics(app: tauri::AppHandle) -> Result<SystemMetricsDto, IpcErrorDto> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        get_system_metrics_impl(&state)
    })
    .await
    .map_err(|_join_err| {
        IpcErrorDto::from(DomainError::InternalError(
            "Failed to complete system metrics collection task".to_string(),
        ))
    })?
}

/// Internal implementation of `get_system_metrics` separated from `State` extraction
/// to permit direct, deterministic unit testing without requiring a Tauri GUI runtime.
pub fn get_system_metrics_impl(state: &AppState) -> Result<SystemMetricsDto, IpcErrorDto> {
    SystemService::get_metrics(state).map_err(IpcErrorDto::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_system_metrics_impl_success() {
        let state = AppState::new("DeskPilot", "0.1.0");
        let result = get_system_metrics_impl(&state)
            .expect("get_system_metrics_impl must succeed on Windows");

        // CPU metrics verification
        assert!(result.cpu.logical_cores > 0);
        assert!(!result.cpu.brand_name.is_empty());
        assert_eq!(result.cpu.architecture, "x86_64");
        assert!(result.cpu.utilization_percent >= 0.0 && result.cpu.utilization_percent <= 100.0);

        // Memory metrics verification
        assert!(result.memory.total_bytes > 0);
        assert!(result.memory.available_bytes > 0);
        assert!(result.memory.total_bytes >= result.memory.available_bytes);
        assert!(result.memory.used_percent >= 0.0 && result.memory.used_percent <= 100.0);

        // Storage metrics verification
        assert!(!result.storage.drives.is_empty());
        for drive in &result.storage.drives {
            assert!(!drive.root_path.is_empty());
            assert!(drive.total_bytes > 0);
            assert!(drive.used_percent >= 0.0 && drive.used_percent <= 100.0);
        }

        // System metadata verification
        assert_eq!(result.system.os_name, "Windows");
        assert_eq!(result.system.architecture, "x86_64");
        assert!(result.system.uptime_seconds > 0);
        assert!(result.sampled_at_ms > 0);
    }

    #[test]
    fn test_system_metrics_worker_join_error_sanitization() {
        let err = DomainError::InternalError(
            "Failed to complete system metrics collection task".to_string(),
        );
        let ipc_err: IpcErrorDto = err.into();
        assert_eq!(ipc_err.code, "INTERNAL_ERROR");
        assert_eq!(
            ipc_err.message,
            "An internal system error occurred. Please consult backend logs."
        );
        assert!(ipc_err.details.is_none());
    }
}
