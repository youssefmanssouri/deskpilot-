use crate::core::ApplicationDiscoveryService;
use crate::error::IpcErrorDto;
use crate::models::{ApplicationDto, DiscoveryResultDto};
use crate::state::AppState;
use tauri::State;

/// Thin Tauri command controller for discovering installed Windows applications.
///
/// Dispatches to `ApplicationDiscoveryService`, updates the privileged in-memory
/// catalog in `AppState`, and maps the domain result into a safe `DiscoveryResultDto`
/// where sensitive machine paths are omitted.
#[tauri::command]
pub fn discover_applications(
    state: State<'_, AppState>,
) -> Result<DiscoveryResultDto, IpcErrorDto> {
    let result = ApplicationDiscoveryService::discover_all().map_err(IpcErrorDto::from)?;

    // Atomically update the in-memory catalog with internal domain models
    state
        .update_catalog(result.applications.clone())
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
