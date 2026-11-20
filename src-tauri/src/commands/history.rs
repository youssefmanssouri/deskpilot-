use crate::core::{LaunchHistoryEntry, LaunchHistoryStore};
use crate::error::IpcErrorDto;
use tauri::State;

/// Returns the recent launches, most recent first. Entries hold only an application id,
/// its display name and the launch time (no paths or arguments).
#[tauri::command]
pub fn get_launch_history(
    history: State<'_, LaunchHistoryStore>,
) -> Result<Vec<LaunchHistoryEntry>, IpcErrorDto> {
    history.entries().map_err(IpcErrorDto::from)
}

/// Deletes all recent launches, in memory and on disk.
#[tauri::command]
pub fn clear_launch_history(history: State<'_, LaunchHistoryStore>) -> Result<(), IpcErrorDto> {
    history.clear().map_err(IpcErrorDto::from)
}
