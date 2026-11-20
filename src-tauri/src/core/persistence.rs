//! Small helper shared by the files DeskPilot writes (launch history, settings).

use std::path::Path;

/// Writes `contents` to `path` atomically: write a temporary file next to it, then
/// rename it over the target, so a crash mid-write never leaves a half-written file.
/// Creates the parent folder when needed.
pub fn write_atomic(path: &Path, contents: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, contents)?;
    std::fs::rename(&tmp, path)
}
