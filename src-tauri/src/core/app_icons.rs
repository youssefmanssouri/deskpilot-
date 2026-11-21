//! Real icons for catalog applications.
//!
//! The frontend asks by catalog ID only; the file to read comes from the in-memory catalog,
//! never from the page. Icons are read from local `.exe` files only (no network shares or
//! device paths, so no network access and no credential prompts), and results, including
//! "no icon", are cached in memory for the session.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::core::ApplicationLauncherService;

/// Icons are extracted at 64 x 64 so they stay sharp on 200% displays at 32 px.
pub const ICON_SIZE: u32 = 64;

/// Byte length of one icon: `ICON_SIZE` x `ICON_SIZE` RGBA pixels.
pub const ICON_BYTES: usize = (ICON_SIZE * ICON_SIZE * 4) as usize;

/// Upper bound on cached entries; the cache is cleared when it is reached. Catalogs are
/// far smaller in practice, so this only guards against unbounded growth.
pub const MAX_CACHED_ICONS: usize = 4096;

/// Returns the local executable whose icon may be read for a catalog `target_path`, or
/// `None` when the target must not be touched (not an `.exe`, network share, device
/// path, missing, or not a regular file). Reading an icon does not run the file, so the
/// launcher's temp/Downloads policy does not apply here.
pub fn resolve_icon_source(raw_target: &str) -> Option<PathBuf> {
    ApplicationLauncherService::validate_lexical_path(raw_target).ok()?;
    let path = Path::new(raw_target);
    if !std::fs::metadata(path).ok()?.is_file() {
        return None;
    }
    let canonical = std::fs::canonicalize(path).ok()?;
    ApplicationLauncherService::validate_canonical_prefix(&canonical).ok()?;
    let clean = ApplicationLauncherService::normalize_verbatim_path(canonical);
    ApplicationLauncherService::validate_target_extension(&clean).ok()?;
    Some(clean)
}

/// Session cache of extracted icons, keyed by application ID. `None` = no usable icon.
#[derive(Debug, Default)]
pub struct AppIconCache {
    entries: Mutex<HashMap<String, Option<Vec<u8>>>>,
}

impl AppIconCache {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the cached result for `id`, or computes it with `load` and caches it.
    /// The lock is not held while `load` runs, so slow files never block other lookups.
    pub fn get_or_load(&self, id: &str, load: impl FnOnce() -> Option<Vec<u8>>) -> Option<Vec<u8>> {
        if let Ok(entries) = self.entries.lock()
            && let Some(hit) = entries.get(id)
        {
            return hit.clone();
        }
        let loaded = load().filter(|pixels| pixels.len() == ICON_BYTES);
        if let Ok(mut entries) = self.entries.lock() {
            if entries.len() >= MAX_CACHED_ICONS {
                entries.clear();
            }
            entries.insert(id.to_string(), loaded.clone());
        }
        loaded
    }

    pub fn len(&self) -> usize {
        self.entries.lock().map(|e| e.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn rejects_network_device_and_non_exe_targets_without_touching_disk() {
        for raw in [
            r"\\server\share\app.exe",
            "//server/share/app.exe",
            r"\\?\UNC\server\share\app.exe",
            r"\\.\PhysicalDrive0",
            r"C:\Apps\script.cmd",
            r"C:\Apps\readme.txt",
            "",
        ] {
            assert_eq!(resolve_icon_source(raw), None, "{raw}");
        }
    }

    #[test]
    fn missing_file_has_no_icon_source() {
        assert_eq!(
            resolve_icon_source(r"C:\DeskPilotTest\does-not-exist\app.exe"),
            None
        );
    }

    #[test]
    fn cache_loads_once_and_remembers_missing_icons() {
        let cache = AppIconCache::new();
        let calls = Cell::new(0);
        let pixels = vec![7u8; ICON_BYTES];

        let first = cache.get_or_load("app_a", || {
            calls.set(calls.get() + 1);
            Some(pixels.clone())
        });
        let second = cache.get_or_load("app_a", || {
            calls.set(calls.get() + 1);
            None
        });
        assert_eq!(first.as_deref(), Some(pixels.as_slice()));
        assert_eq!(second, first);
        assert_eq!(calls.get(), 1);

        assert_eq!(cache.get_or_load("app_b", || None), None);
        assert_eq!(cache.get_or_load("app_b", || Some(pixels.clone())), None);
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn cache_rejects_wrong_sized_pixel_buffers() {
        let cache = AppIconCache::new();
        assert_eq!(cache.get_or_load("app_c", || Some(vec![1, 2, 3])), None);
    }

    #[test]
    fn cache_is_bounded() {
        let cache = AppIconCache::new();
        for i in 0..MAX_CACHED_ICONS {
            cache.get_or_load(&format!("app_{i}"), || None);
        }
        assert_eq!(cache.len(), MAX_CACHED_ICONS);
        cache.get_or_load("one_more", || None);
        assert_eq!(cache.len(), 1);
    }
}
