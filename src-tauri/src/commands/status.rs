use crate::core::StatusService;
use crate::error::IpcErrorDto;
use crate::models::{AppStatusDto, CommandIntentDto};
use crate::state::AppState;
use tauri::State;

/// Thin Tauri command controller for querying application status.
///
/// Dispatches to `StatusService` and maps any domain failure into an `IpcErrorDto`.
#[tauri::command]
pub fn get_app_status(state: State<'_, AppState>) -> Result<AppStatusDto, IpcErrorDto> {
    StatusService::get_status(&state)
        .map(AppStatusDto::from)
        .map_err(IpcErrorDto::from)
}

/// Thin Tauri command controller for validating frontend command intents.
///
/// Validates the structure and policy conformance of incoming intent DTOs.
#[tauri::command]
pub fn validate_command_intent(intent: CommandIntentDto) -> Result<(), IpcErrorDto> {
    StatusService::validate_intent(&intent).map_err(IpcErrorDto::from)
}
