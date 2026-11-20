//! User settings, stored in `settings.json` in DeskPilot's app-data folder.
//!
//! - Only preferences are stored: theme, launch-history switch, launcher shortcut and the
//!   glass effect. "Start with Windows" is not stored here: Windows itself (DeskPilot's
//!   value in the current user's Run key) is the source of truth.
//! - The file is treated as untrusted: each field is read on its own, and a missing or
//!   invalid field falls back to its default instead of discarding the whole file.
//! - No Tauri or Windows dependencies, so it is unit-testable on any platform.

use crate::core::persistence::write_atomic;
use crate::error::DomainError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

/// File name inside the app-data directory.
pub const SETTINGS_FILE_NAME: &str = "settings.json";

/// Default launcher shortcut (canonical form, parsed by tauri-plugin-global-shortcut).
pub const DEFAULT_SHORTCUT: &str = "Ctrl+Alt+Space";

/// First Windows build number of Windows 11 (Mica requires Windows 11).
pub const WINDOWS_11_FIRST_BUILD: u32 = 22000;

const FORMAT_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemePreference {
    /// Follow the Windows app theme.
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub theme: ThemePreference,
    pub history_enabled: bool,
    /// Canonical shortcut, e.g. "Ctrl+Alt+Space" or "Ctrl+Super+K".
    pub launcher_shortcut: String,
    /// User preference; only takes effect on Windows 11, after a restart.
    pub glass_effect: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemePreference::System,
            history_enabled: true,
            launcher_shortcut: DEFAULT_SHORTCUT.to_string(),
            glass_effect: false,
        }
    }
}

/// Reads settings from a parsed JSON value, field by field. Anything missing or invalid
/// keeps its default, so one bad value never resets the others.
pub fn settings_from_json(value: &Value) -> Settings {
    let mut s = Settings::default();
    let Some(obj) = value.get("settings").and_then(Value::as_object) else {
        return s;
    };
    if let Some(theme) = obj
        .get("theme")
        .and_then(|v| serde_json::from_value::<ThemePreference>(v.clone()).ok())
    {
        s.theme = theme;
    }
    if let Some(b) = obj.get("historyEnabled").and_then(Value::as_bool) {
        s.history_enabled = b;
    }
    if let Some(shortcut) = obj
        .get("launcherShortcut")
        .and_then(Value::as_str)
        .and_then(|raw| normalize_shortcut(raw).ok())
    {
        s.launcher_shortcut = shortcut;
    }
    if let Some(b) = obj.get("glassEffect").and_then(Value::as_bool) {
        s.glass_effect = b;
    }
    s
}

/// Validates a shortcut and returns its canonical form ("Ctrl+Alt+Shift+Super+Key").
///
/// Rules (stricter than the plugin's parser on purpose):
/// - Modifiers: Ctrl, Alt, Shift, Win/Super (any order, case-insensitive, no duplicates).
/// - At least one of Ctrl, Alt or Win, so the shortcut never hijacks normal typing.
/// - Exactly one key: A-Z, 0-9, Space or F1-F24.
pub fn normalize_shortcut(input: &str) -> Result<String, DomainError> {
    let invalid = |why: &str| DomainError::InvalidInput(format!("Invalid shortcut: {why}"));
    let tokens: Vec<String> = input
        .split('+')
        .map(|t| t.trim().to_ascii_uppercase())
        .collect();
    if tokens.iter().any(String::is_empty) {
        return Err(invalid("it has an empty part."));
    }

    let (mut ctrl, mut alt, mut shift, mut win) = (false, false, false, false);
    let mut key: Option<String> = None;
    for token in &tokens {
        let flag = match token.as_str() {
            "CTRL" | "CONTROL" => &mut ctrl,
            "ALT" => &mut alt,
            "SHIFT" => &mut shift,
            "WIN" | "SUPER" | "META" => &mut win,
            _ => {
                if key.is_some() {
                    return Err(invalid("use one key plus modifiers."));
                }
                key = Some(canonical_key(token).ok_or_else(|| {
                    invalid("use a letter, a digit, Space or F1-F24 as the key.")
                })?);
                continue;
            }
        };
        if *flag {
            return Err(invalid("a modifier is repeated."));
        }
        *flag = true;
    }
    let key = key.ok_or_else(|| invalid("add a key after the modifiers."))?;
    if !(ctrl || alt || win) {
        return Err(invalid(
            "include Ctrl, Alt or Win so it doesn't block normal typing.",
        ));
    }

    let mut parts: Vec<&str> = Vec::with_capacity(5);
    if ctrl {
        parts.push("Ctrl");
    }
    if alt {
        parts.push("Alt");
    }
    if shift {
        parts.push("Shift");
    }
    if win {
        parts.push("Super");
    }
    parts.push(&key);
    Ok(parts.join("+"))
}

fn canonical_key(token: &str) -> Option<String> {
    if token == "SPACE" {
        return Some("Space".to_string());
    }
    if token.len() == 1 {
        let c = token.chars().next()?;
        if c.is_ascii_uppercase() || c.is_ascii_digit() {
            return Some(c.to_string());
        }
        return None;
    }
    // "F1".."F24" only; rejects "F01", "F25", "F+".
    let n = token.strip_prefix('F')?.parse::<u8>().ok()?;
    ((1..=24).contains(&n) && n.to_string() == token[1..]).then(|| format!("F{n}"))
}

/// Display form of a canonical shortcut: "Super" is shown as "Win" on Windows.
pub fn shortcut_label(canonical: &str) -> String {
    canonical
        .split('+')
        .map(|p| if p == "Super" { "Win" } else { p })
        .collect::<Vec<_>>()
        .join("+")
}

/// Command stored in the Windows startup entry: the executable path in quotes (paths
/// under "Program Files" contain spaces) followed by the argument.
pub fn startup_command(executable: &Path, argument: &str) -> String {
    format!("\"{}\" {}", executable.display(), argument)
}

/// Mica is only available on Windows 11 (build 22000 and later). Unknown build = no.
pub fn supports_glass(windows_build: Option<u32>) -> bool {
    windows_build.is_some_and(|b| b >= WINDOWS_11_FIRST_BUILD)
}

/// Thread-safe settings with file persistence. Registered in memory when the app is
/// built and attached to its file in `setup`, like `LaunchHistoryStore`.
#[derive(Debug)]
pub struct SettingsStore {
    path: OnceLock<PathBuf>,
    settings: Mutex<Settings>,
}

impl Default for SettingsStore {
    fn default() -> Self {
        Self::in_memory()
    }
}

impl SettingsStore {
    pub fn in_memory() -> Self {
        Self {
            path: OnceLock::new(),
            settings: Mutex::new(Settings::default()),
        }
    }

    /// Loads settings from `path` (missing or corrupt file = defaults) and keeps the path
    /// for later saves. Can only be attached once.
    pub fn attach_file(&self, path: PathBuf) -> Result<Settings, DomainError> {
        let mut guard = self.lock()?;
        if self.path.get().is_some() {
            return Err(DomainError::InvalidInput(
                "Settings are already attached to a file.".to_string(),
            ));
        }
        let loaded = Self::read_file(&path).unwrap_or_default();
        let _ = self.path.set(path);
        *guard = loaded.clone();
        Ok(loaded)
    }

    fn read_file(path: &Path) -> Option<Settings> {
        let raw = std::fs::read_to_string(path).ok()?;
        let value: Value = serde_json::from_str(&raw).ok()?;
        if value.get("version").and_then(Value::as_u64) != Some(FORMAT_VERSION as u64) {
            return None;
        }
        Some(settings_from_json(&value))
    }

    pub fn get(&self) -> Result<Settings, DomainError> {
        Ok(self.lock()?.clone())
    }

    /// Applies `change` and saves. If saving fails, the in-memory settings are left unchanged.
    pub fn update(&self, change: impl FnOnce(&mut Settings)) -> Result<Settings, DomainError> {
        let mut guard = self.lock()?;
        let mut next = guard.clone();
        change(&mut next);
        self.persist(&next)?;
        *guard = next.clone();
        Ok(next)
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Settings>, DomainError> {
        self.settings
            .lock()
            .map_err(|_| DomainError::InternalError("Settings lock poisoned".to_string()))
    }

    fn persist(&self, settings: &Settings) -> Result<(), DomainError> {
        let Some(path) = self.path.get() else {
            return Ok(());
        };
        let json = serde_json::to_string_pretty(&serde_json::json!({
            "version": FORMAT_VERSION,
            "settings": settings,
        }))
        .map_err(|_| DomainError::InternalError("Could not serialize settings.".to_string()))?;
        write_atomic(path, &json)
            .map_err(|_| DomainError::OperationFailed("Could not save settings.".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn temp_path(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "deskpilot-settings-test-{}-{}",
            name,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join(SETTINGS_FILE_NAME)
    }

    #[test]
    fn normalizes_valid_shortcuts() {
        assert_eq!(
            normalize_shortcut("ctrl+alt+space").unwrap(),
            "Ctrl+Alt+Space"
        );
        assert_eq!(
            normalize_shortcut(" Alt + Ctrl + k ").unwrap(),
            "Ctrl+Alt+K"
        );
        assert_eq!(
            normalize_shortcut("win+shift+F13").unwrap(),
            "Shift+Super+F13"
        );
        assert_eq!(normalize_shortcut("Control+7").unwrap(), "Ctrl+7");
        assert_eq!(normalize_shortcut("super+space").unwrap(), "Super+Space");
    }

    #[test]
    fn rejects_unsafe_or_malformed_shortcuts() {
        for bad in [
            "",
            "space",
            "a",
            "shift+a",
            "ctrl+",
            "ctrl++a",
            "ctrl+ctrl+a",
            "ctrl+a+b",
            "ctrl+alt",
            "ctrl+F25",
            "ctrl+F01",
            "ctrl+enter",
            "ctrl+é",
        ] {
            assert!(normalize_shortcut(bad).is_err(), "should reject {bad:?}");
        }
    }

    #[test]
    fn label_shows_win_for_super() {
        assert_eq!(shortcut_label("Ctrl+Super+K"), "Ctrl+Win+K");
        assert_eq!(shortcut_label("Ctrl+Alt+Space"), "Ctrl+Alt+Space");
    }

    #[test]
    fn startup_command_quotes_the_path() {
        let exe = Path::new(r"C:\Program Files\DeskPilot\deskpilot.exe");
        assert_eq!(
            startup_command(exe, "--autostart"),
            r#""C:\Program Files\DeskPilot\deskpilot.exe" --autostart"#
        );
    }

    #[test]
    fn glass_needs_windows_11() {
        assert!(!supports_glass(None));
        assert!(!supports_glass(Some(19045)));
        assert!(supports_glass(Some(22000)));
        assert!(supports_glass(Some(26100)));
    }

    #[test]
    fn reads_fields_independently_and_falls_back_per_field() {
        let s = settings_from_json(&json!({
            "version": 1,
            "settings": {
                "theme": "purple",
                "historyEnabled": false,
                "launcherShortcut": "shift+a",
                "glassEffect": true,
                "unknownFutureField": 42
            }
        }));
        assert_eq!(s.theme, ThemePreference::System);
        assert!(!s.history_enabled);
        assert_eq!(s.launcher_shortcut, DEFAULT_SHORTCUT);
        assert!(s.glass_effect);
    }

    #[test]
    fn update_persists_and_reloads() {
        let path = temp_path("roundtrip");
        let store = SettingsStore::in_memory();
        assert_eq!(
            store.attach_file(path.clone()).unwrap(),
            Settings::default()
        );

        store
            .update(|s| {
                s.theme = ThemePreference::Dark;
                s.launcher_shortcut = "Ctrl+Super+K".to_string();
            })
            .unwrap();

        let reloaded = SettingsStore::in_memory();
        let s = reloaded.attach_file(path.clone()).unwrap();
        assert_eq!(s.theme, ThemePreference::Dark);
        assert_eq!(s.launcher_shortcut, "Ctrl+Super+K");
        assert!(reloaded.attach_file(path.clone()).is_err());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn corrupt_or_unknown_version_files_give_defaults() {
        let path = temp_path("corrupt");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, "{ not json").unwrap();
        assert_eq!(
            SettingsStore::in_memory()
                .attach_file(path.clone())
                .unwrap(),
            Settings::default()
        );
        std::fs::write(&path, r#"{"version": 2, "settings": {"theme": "dark"}}"#).unwrap();
        assert_eq!(
            SettingsStore::in_memory()
                .attach_file(path.clone())
                .unwrap(),
            Settings::default()
        );
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn in_memory_store_updates_without_a_file() {
        let store = SettingsStore::in_memory();
        let s = store.update(|s| s.history_enabled = false).unwrap();
        assert!(!s.history_enabled);
        assert!(!store.get().unwrap().history_enabled);
    }
}
