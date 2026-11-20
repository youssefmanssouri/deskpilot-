//! Local launch history ("Recent apps").
//!
//! Privacy and safety boundaries:
//! - Stores only the opaque application id, its display name and the launch time.
//!   Never paths, arguments, working directories or usernames.
//! - Lives in a single JSON file in DeskPilot's app-data folder; nothing leaves the machine.
//! - Entries are recorded by the backend after a successful launch, never supplied by
//!   the frontend, so page scripts cannot inject arbitrary history.
//! - The file is treated as untrusted on load: every entry is re-validated, and a
//!   missing or corrupt file yields an empty history instead of an error.
//!
//! This module has no Tauri or Windows dependencies so its logic can be unit-tested
//! on any platform.

use crate::error::DomainError;
use crate::security::InputGuard;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// Maximum number of entries kept (most recent first).
pub const MAX_HISTORY_ENTRIES: usize = 10;

/// Maximum stored length of a display name, in characters.
pub const MAX_DISPLAY_NAME_CHARS: usize = 256;

/// File name inside the app-data directory.
pub const HISTORY_FILE_NAME: &str = "launch-history.json";

/// On-disk format version. Bump when the schema changes.
const FORMAT_VERSION: u32 = 1;

/// One recent launch, as stored on disk and sent over IPC.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LaunchHistoryEntry {
    pub application_id: String,
    pub display_name: String,
    /// Milliseconds since the Unix epoch.
    pub launched_at_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct HistoryFile {
    version: u32,
    entries: Vec<LaunchHistoryEntry>,
}

/// Removes control characters, trims, and caps the length of a display name.
/// Returns `None` when nothing printable remains.
pub fn sanitize_display_name(raw: &str) -> Option<String> {
    let cleaned: String = raw
        .chars()
        .filter(|c| !c.is_control())
        .collect::<String>()
        .trim()
        .chars()
        .take(MAX_DISPLAY_NAME_CHARS)
        .collect();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

/// Validates and normalizes a single entry. Returns `None` for entries that must be dropped.
fn sanitize_entry(entry: LaunchHistoryEntry) -> Option<LaunchHistoryEntry> {
    InputGuard::validate_application_id(&entry.application_id).ok()?;
    let display_name = sanitize_display_name(&entry.display_name)?;
    Some(LaunchHistoryEntry {
        application_id: entry.application_id,
        display_name,
        launched_at_ms: entry.launched_at_ms,
    })
}

/// Normalizes a list read from disk: drops invalid entries and duplicates (keeping the
/// first, i.e. most recent, occurrence) and caps the length.
pub fn sanitize_entries(entries: Vec<LaunchHistoryEntry>) -> Vec<LaunchHistoryEntry> {
    let mut out: Vec<LaunchHistoryEntry> = Vec::with_capacity(MAX_HISTORY_ENTRIES);
    for entry in entries.into_iter().filter_map(sanitize_entry) {
        if out.iter().any(|e| e.application_id == entry.application_id) {
            continue;
        }
        out.push(entry);
        if out.len() == MAX_HISTORY_ENTRIES {
            break;
        }
    }
    out
}

/// Returns a new list with `entry` first, any older entry for the same app removed,
/// and the length capped at `MAX_HISTORY_ENTRIES`.
pub fn push_entry(
    entries: &[LaunchHistoryEntry],
    entry: LaunchHistoryEntry,
) -> Vec<LaunchHistoryEntry> {
    let mut out = Vec::with_capacity(MAX_HISTORY_ENTRIES);
    let id = entry.application_id.clone();
    out.push(entry);
    out.extend(
        entries
            .iter()
            .filter(|e| e.application_id != id)
            .take(MAX_HISTORY_ENTRIES - 1)
            .cloned(),
    );
    out
}

/// Thread-safe launch history with optional file persistence.
///
/// Registered with Tauri as an in-memory store when the app is built (so commands can
/// never hit unmanaged state), then attached to its file in `setup` once the app-data
/// path is known.
#[derive(Debug)]
pub struct LaunchHistoryStore {
    path: OnceLock<PathBuf>,
    entries: Mutex<Vec<LaunchHistoryEntry>>,
}

impl LaunchHistoryStore {
    /// History kept only in memory until `attach_file` is called (or for the whole session
    /// when no app-data folder is available, and in tests).
    pub fn in_memory() -> Self {
        Self {
            path: OnceLock::new(),
            entries: Mutex::new(Vec::new()),
        }
    }

    /// Loads history from `path`. A missing, unreadable, corrupt or wrong-version file yields
    /// an empty history; it is never an error, so a bad file cannot stop the app from starting.
    pub fn load(path: PathBuf) -> Self {
        let store = Self::in_memory();
        // A fresh store has no path yet, so attaching cannot fail.
        let _ = store.attach_file(path);
        store
    }

    /// Attaches persistence to an in-memory store: loads the file, keeps any launches
    /// recorded before attaching (newest first), and saves the merged list.
    /// Can only be done once; later calls return an error and change nothing.
    pub fn attach_file(&self, path: PathBuf) -> Result<(), DomainError> {
        let mut guard = self.lock()?;
        if self.path.get().is_some() {
            return Err(DomainError::InvalidInput(
                "Launch history is already attached to a file.".to_string(),
            ));
        }
        let mut merged = Self::read_file(&path).unwrap_or_default();
        for early in guard.iter().rev() {
            merged = push_entry(&merged, early.clone());
        }
        let had_early_entries = !guard.is_empty();
        let _ = self.path.set(path);
        if had_early_entries {
            self.persist(&merged)?;
        }
        *guard = merged;
        Ok(())
    }

    fn read_file(path: &Path) -> Option<Vec<LaunchHistoryEntry>> {
        let raw = std::fs::read_to_string(path).ok()?;
        let file: HistoryFile = serde_json::from_str(&raw).ok()?;
        if file.version != FORMAT_VERSION {
            return None;
        }
        Some(sanitize_entries(file.entries))
    }

    /// Current entries, most recent first.
    pub fn entries(&self) -> Result<Vec<LaunchHistoryEntry>, DomainError> {
        Ok(self.lock()?.clone())
    }

    /// Records a successful launch and persists the result.
    pub fn record(
        &self,
        application_id: &str,
        display_name: &str,
        launched_at_ms: u64,
    ) -> Result<(), DomainError> {
        InputGuard::validate_application_id(application_id)?;
        let display_name = sanitize_display_name(display_name).ok_or_else(|| {
            DomainError::InvalidInput("Display name is empty after sanitization.".to_string())
        })?;
        let entry = LaunchHistoryEntry {
            application_id: application_id.to_string(),
            display_name,
            launched_at_ms,
        };

        let mut guard = self.lock()?;
        let updated = push_entry(&guard, entry);
        self.persist(&updated)?;
        *guard = updated;
        Ok(())
    }

    /// Removes all entries and persists the empty history.
    pub fn clear(&self) -> Result<(), DomainError> {
        let mut guard = self.lock()?;
        self.persist(&[])?;
        guard.clear();
        Ok(())
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Vec<LaunchHistoryEntry>>, DomainError> {
        self.entries
            .lock()
            .map_err(|_| DomainError::InternalError("Launch history lock poisoned".to_string()))
    }

    /// Atomic write: serialize to a temporary file next to the target, then rename over it,
    /// so a crash mid-write never leaves a half-written history file.
    fn persist(&self, entries: &[LaunchHistoryEntry]) -> Result<(), DomainError> {
        let Some(path) = self.path.get() else {
            return Ok(());
        };
        let io_err = |_| DomainError::OperationFailed("Could not save launch history.".to_string());

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(io_err)?;
        }
        let file = HistoryFile {
            version: FORMAT_VERSION,
            entries: entries.to_vec(),
        };
        let json = serde_json::to_string_pretty(&file).map_err(|_| {
            DomainError::InternalError("Could not serialize launch history.".to_string())
        })?;
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, json).map_err(io_err)?;
        std::fs::rename(&tmp, path).map_err(io_err)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u8) -> String {
        format!("app_{:032x}", n)
    }

    fn entry(n: u8, at: u64) -> LaunchHistoryEntry {
        LaunchHistoryEntry {
            application_id: id(n),
            display_name: format!("App {n}"),
            launched_at_ms: at,
        }
    }

    fn temp_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "deskpilot-history-test-{}-{}",
            name,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join(HISTORY_FILE_NAME)
    }

    #[test]
    fn push_entry_puts_newest_first_and_removes_duplicates() {
        let list = vec![entry(1, 10), entry(2, 20), entry(3, 30)];
        let out = push_entry(&list, entry(2, 99));
        let ids: Vec<_> = out.iter().map(|e| e.application_id.clone()).collect();
        assert_eq!(ids, vec![id(2), id(1), id(3)]);
        assert_eq!(out[0].launched_at_ms, 99);
    }

    #[test]
    fn push_entry_caps_length() {
        let list: Vec<_> = (1..=10).map(|n| entry(n, n as u64)).collect();
        let out = push_entry(&list, entry(42, 1000));
        assert_eq!(out.len(), MAX_HISTORY_ENTRIES);
        assert_eq!(out[0].application_id, id(42));
        assert!(!out.iter().any(|e| e.application_id == id(10)));
    }

    #[test]
    fn sanitize_display_name_strips_control_chars_and_caps_length() {
        assert_eq!(
            sanitize_display_name("  Code\u{0007}\n "),
            Some("Code".to_string())
        );
        assert_eq!(sanitize_display_name("\u{0000}\t "), None);
        let long = "x".repeat(MAX_DISPLAY_NAME_CHARS + 50);
        assert_eq!(
            sanitize_display_name(&long).unwrap().chars().count(),
            MAX_DISPLAY_NAME_CHARS
        );
    }

    #[test]
    fn sanitize_entries_drops_invalid_and_duplicate_entries() {
        let bad_id = LaunchHistoryEntry {
            application_id: "C:\\Windows\\evil.exe".to_string(),
            display_name: "Evil".to_string(),
            launched_at_ms: 1,
        };
        let empty_name = LaunchHistoryEntry {
            application_id: id(5),
            display_name: "\u{0001}".to_string(),
            launched_at_ms: 2,
        };
        let out = sanitize_entries(vec![
            entry(1, 50),
            bad_id,
            entry(1, 40),
            empty_name,
            entry(2, 30),
        ]);
        let ids: Vec<_> = out.iter().map(|e| e.application_id.clone()).collect();
        assert_eq!(ids, vec![id(1), id(2)]);
        assert_eq!(out[0].launched_at_ms, 50);
    }

    #[test]
    fn record_rejects_invalid_ids_and_empty_names() {
        let store = LaunchHistoryStore::in_memory();
        assert!(store.record("not-an-id", "App", 1).is_err());
        assert!(store.record(&id(1), " \u{0007} ", 1).is_err());
        assert!(store.entries().unwrap().is_empty());
    }

    #[test]
    fn record_and_clear_round_trip_through_the_file() {
        let path = temp_path("roundtrip");
        let store = LaunchHistoryStore::load(path.clone());
        assert!(store.entries().unwrap().is_empty());

        store.record(&id(1), "Visual Studio Code", 100).unwrap();
        store.record(&id(2), "Windows Terminal", 200).unwrap();
        store.record(&id(1), "Visual Studio Code", 300).unwrap();

        let reloaded = LaunchHistoryStore::load(path.clone());
        let entries = reloaded.entries().unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].application_id, id(1));
        assert_eq!(entries[0].launched_at_ms, 300);
        assert_eq!(entries[1].display_name, "Windows Terminal");
        assert!(
            !path.with_extension("json.tmp").exists(),
            "temp file must be renamed away"
        );

        reloaded.clear().unwrap();
        assert!(
            LaunchHistoryStore::load(path.clone())
                .entries()
                .unwrap()
                .is_empty()
        );
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn load_survives_corrupt_and_unknown_version_files() {
        let path = temp_path("corrupt");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();

        std::fs::write(&path, "{ this is not json").unwrap();
        assert!(
            LaunchHistoryStore::load(path.clone())
                .entries()
                .unwrap()
                .is_empty()
        );

        std::fs::write(&path, r#"{"version": 99, "entries": []}"#).unwrap();
        assert!(
            LaunchHistoryStore::load(path.clone())
                .entries()
                .unwrap()
                .is_empty()
        );

        // A tampered file is sanitized, not trusted.
        let tampered = format!(
            r#"{{"version":1,"entries":[{{"applicationId":"../../etc","displayName":"x","launchedAtMs":1}},{{"applicationId":"{}","displayName":"Ok","launchedAtMs":2}}]}}"#,
            id(7)
        );
        std::fs::write(&path, tampered).unwrap();
        let entries = LaunchHistoryStore::load(path.clone()).entries().unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].application_id, id(7));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn attach_file_merges_early_launches_and_only_attaches_once() {
        let path = temp_path("attach");
        LaunchHistoryStore::load(path.clone())
            .record(&id(1), "Old", 10)
            .unwrap();

        let store = LaunchHistoryStore::in_memory();
        store.record(&id(2), "Early", 20).unwrap();
        store.attach_file(path.clone()).unwrap();

        let ids: Vec<_> = store
            .entries()
            .unwrap()
            .iter()
            .map(|e| e.application_id.clone())
            .collect();
        assert_eq!(ids, vec![id(2), id(1)]);
        // The merged list was saved.
        assert_eq!(
            LaunchHistoryStore::load(path.clone())
                .entries()
                .unwrap()
                .len(),
            2
        );

        assert!(store.attach_file(temp_path("other")).is_err());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn in_memory_store_never_touches_disk() {
        let store = LaunchHistoryStore::in_memory();
        store.record(&id(3), "App", 5).unwrap();
        assert_eq!(store.entries().unwrap().len(), 1);
        store.clear().unwrap();
        assert!(store.entries().unwrap().is_empty());
    }
}
