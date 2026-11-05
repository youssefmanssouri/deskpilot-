use crate::core::ProcessService;
use crate::error::{DomainError, IpcErrorDto};
use crate::models::ProcessListResultDto;
use crate::state::AppState;
use tauri::Manager;

/// Thin Tauri command controller for enumerating running Windows processes.
///
/// Dispatches execution to a dedicated blocking worker thread pool via `tauri::async_runtime::spawn_blocking`,
/// ensuring synchronous Win32 Toolhelp32 snapshot traversal and process handle inspection never block
/// Tokio async workers or the application UI thread.
///
/// Accepts no user-controlled process IDs, executable paths, shell arguments, or OS instructions.
#[tauri::command]
pub async fn list_processes(app: tauri::AppHandle) -> Result<ProcessListResultDto, IpcErrorDto> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        list_processes_impl(&state)
    })
    .await
    .map_err(|_join_err| {
        IpcErrorDto::from(DomainError::InternalError(
            "Failed to complete process enumeration task".to_string(),
        ))
    })?
}

/// Internal implementation of `list_processes` separated from `State` extraction
/// to permit direct, deterministic unit testing without requiring a Tauri GUI runtime.
pub fn list_processes_impl(state: &AppState) -> Result<ProcessListResultDto, IpcErrorDto> {
    ProcessService::list_processes(state).map_err(IpcErrorDto::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_list_processes_impl_success() {
        let state = AppState::new("DeskPilot", "0.1.0");
        let result =
            list_processes_impl(&state).expect("list_processes_impl should succeed on Windows");

        // The running process snapshot walk must enumerate at least one active process (e.g. self)
        assert!(result.total_processes > 0);
        assert!(!result.processes.is_empty());
        assert!(result.captured_at_ms > 0);
        assert!(result.processes.len() <= 2000);
        assert_eq!(
            result.is_truncated,
            result.total_processes > result.processes.len()
        );

        // Each process record must possess a valid identity token and non-empty name
        for proc in &result.processes {
            assert!(!proc.id.is_empty());
            assert!(!proc.name.is_empty());
        }
    }

    #[test]
    fn test_list_processes_overflow_error_mapping() {
        let state = AppState::new("DeskPilot", "0.1.0");
        // Exhaust the snapshot sequence generator to u64::MAX
        state.set_process_snapshot_seq_for_testing(u64::MAX);

        let result = list_processes_impl(&state);
        assert!(result.is_err());
        let ipc_err = result.err().unwrap();
        assert_eq!(ipc_err.code, "OPERATION_FAILED");
        assert!(
            ipc_err
                .message
                .contains("Process snapshot sequence numbers exhausted")
        );
    }

    #[test]
    fn test_process_list_result_serialization_camel_case() {
        use crate::models::{ProcessDto, ProcessQueryStatus};

        let dto = ProcessListResultDto {
            processes: vec![ProcessDto {
                id: "v1:100:01db2d5843a0e120".to_string(),
                pid: 100,
                parent_pid: 50,
                name: "test.exe".to_string(),
                executable_path: Some(r"C:\Windows\System32\test.exe".to_string()),
                created_at: Some("2026-11-03T12:00:00.000Z".to_string()),
                working_set_bytes: Some(10485760),
                cpu_time_ms: Some(250),
                session_id: Some(1),
                status: ProcessQueryStatus::Full,
            }],
            total_processes: 1,
            captured_at_ms: 1730678400000,
            is_truncated: false,
        };

        let json = serde_json::to_string(&dto).expect("Serialization must succeed");
        assert!(json.contains(r#""processes":["#));
        assert!(json.contains(r#""totalProcesses":1"#));
        assert!(json.contains(r#""capturedAtMs":1730678400000"#));
        assert!(json.contains(r#""isTruncated":false"#));
        assert!(json.contains(r#""parentPid":50"#));
        assert!(json.contains(r#""workingSetBytes":10485760"#));
        assert!(json.contains(r#""cpuTimeMs":250"#));
        assert!(json.contains(r#""sessionId":1"#));
        assert!(json.contains(r#""status":"full""#));
    }

    #[test]
    fn test_list_processes_worker_join_error_sanitization() {
        let err =
            DomainError::InternalError("Failed to complete process enumeration task".to_string());
        let ipc_err: IpcErrorDto = err.into();
        assert_eq!(ipc_err.code, "INTERNAL_ERROR");
        assert_eq!(
            ipc_err.message,
            "An internal system error occurred. Please consult backend logs."
        );
        assert!(ipc_err.details.is_none());
    }
}
