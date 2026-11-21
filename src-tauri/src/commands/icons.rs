use tauri::ipc::Response;
use tauri::{AppHandle, Manager};

use crate::core::app_icons::{AppIconCache, ICON_SIZE, resolve_icon_source};
use crate::error::{DomainError, IpcErrorDto};
use crate::security::InputGuard;
use crate::state::AppState;

/// Returns the real icon of a catalog application as raw RGBA bytes (`ICON_SIZE` x
/// `ICON_SIZE`, top row first), or an empty buffer when it has no usable icon.
///
/// Takes a catalog ID only: the file is looked up in the in-memory catalog, checked by
/// `resolve_icon_source`, and read on a blocking worker so the UI never waits on disk.
/// Binary response avoids base64 and keeps the Content Security Policy unchanged.
#[tauri::command]
pub async fn get_application_icon(
    app: AppHandle,
    application_id: String,
) -> Result<Response, IpcErrorDto> {
    InputGuard::validate_application_id(&application_id).map_err(IpcErrorDto::from)?;

    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AppState>();
        let cache = app.state::<AppIconCache>();
        // Unknown IDs are not cached: the catalog may simply not be loaded yet.
        let Some(application) = state
            .get_application(&application_id)
            .map_err(IpcErrorDto::from)?
        else {
            return Ok(Response::new(Vec::new()));
        };
        let target = application.target_path;

        let pixels = cache.get_or_load(&application_id, || {
            let source = resolve_icon_source(target.as_deref()?)?;
            crate::platform::extract_icon_rgba(&source, ICON_SIZE)
        });
        Ok(Response::new(pixels.unwrap_or_default()))
    })
    .await
    .map_err(|_| {
        IpcErrorDto::from(DomainError::InternalError(
            "Failed to complete icon extraction task".to_string(),
        ))
    })?
}
