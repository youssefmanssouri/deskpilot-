use crate::core::ApplicationDiscoveryService;
use crate::error::IpcErrorDto;
use crate::models::{ApplicationDto, DiscoveryResultDto};

/// Thin Tauri command controller for discovering installed Windows applications.
///
/// Dispatches to `ApplicationDiscoveryService` and maps the domain result
/// into a safe `DiscoveryResultDto` where sensitive machine paths are omitted.
#[tauri::command]
pub fn discover_applications() -> Result<DiscoveryResultDto, IpcErrorDto> {
    let result = ApplicationDiscoveryService::discover_all().map_err(IpcErrorDto::from)?;

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
