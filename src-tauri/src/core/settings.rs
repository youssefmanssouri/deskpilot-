//! User settings, stored in `settings.json` in DeskPilot's app-data folder.
//!
//! - Only preferences are stored: theme, launch-history switch, launcher shortcut and the
//!   glass effect. "Start with Windows" is not stored here: Windows itself (DeskPilot's
//!   value in the current user's Run key) is the source of truth.
//! - The file is treated as untrusted: each field is read on its own, and a missing or
//!   invalid field falls back to its default instead of discarding the whole file.
//! - No Tauri or Windows dependencies, so it is unit-testable on any platform.

use crate::core::ApplicationLauncherService;
use crate::core::persistence::write_atomic;
use crate::error::DomainError;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Component, Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::{fmt, io};

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

/// Classification of DeskPilot's optional per-user startup entry (`HKCU\...\Run\DeskPilot`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupState {
    /// Neither the `Run` subkey nor the `DeskPilot` value exists.
    Disabled,
    /// Registered with `"<current_exe>" --autostart` for the active executable.
    Enabled,
    /// Registered in DeskPilot's command format for a different DeskPilot executable path
    /// whose target no longer exists on disk (such as a moved installation).
    Stale,
    /// Registered in DeskPilot's command format for another existing DeskPilot executable.
    OtherInstall,
    /// Entry is malformed, uses an unsupported registry value type, carries unexpected
    /// arguments, or points to an unrelated or disallowed target.
    Unrecognized,
}

/// Explicit future resolution operations on non-active DeskPilot registrations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupResolutionAction {
    UpdateStale,
    ClearStale,
    SwitchToCurrent,
}

/// Raw registry read outcome from a [`StartupRegistry`] store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupRegistryValue {
    /// Strict `REG_SZ` string value.
    String(String),
    /// Non-`REG_SZ` value type (such as `REG_DWORD`, `REG_EXPAND_SZ`, `REG_BINARY`) or invalid UTF-16.
    UnsupportedType,
}

/// Errors produced when inspecting or mutating the startup registration.
/// Messages never include registry key paths, raw command values, or executable file paths.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupMutationError {
    /// Underlying operating-system permission failure.
    PermissionDenied,
    /// An existing registration cannot be overwritten or deleted safely under the requested action.
    Conflict(StartupState),
    /// Internal path or command invariant failure.
    InvalidState(&'static str),
    /// Other genuine registry or I/O error.
    IoFailure(io::ErrorKind),
}

impl fmt::Display for StartupMutationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PermissionDenied => {
                write!(
                    f,
                    "Permission denied while accessing Windows startup settings."
                )
            }
            Self::Conflict(StartupState::Stale) => write!(
                f,
                "A stale DeskPilot startup entry already exists and requires explicit repair."
            ),
            Self::Conflict(StartupState::OtherInstall) => write!(
                f,
                "Another DeskPilot installation is currently configured to start with Windows."
            ),
            Self::Conflict(StartupState::Unrecognized) => write!(
                f,
                "An unrecognized startup entry already exists and will not be modified."
            ),
            Self::Conflict(_) => {
                write!(f, "The startup entry state does not permit that operation.")
            }
            Self::InvalidState(msg) => write!(f, "{msg}"),
            Self::IoFailure(_) => write!(f, "Could not access Windows startup settings."),
        }
    }
}

impl std::error::Error for StartupMutationError {}

impl From<io::Error> for StartupMutationError {
    fn from(err: io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::PermissionDenied => Self::PermissionDenied,
            kind => Self::IoFailure(kind),
        }
    }
}

impl From<StartupMutationError> for io::Error {
    fn from(err: StartupMutationError) -> Self {
        let kind = match err {
            StartupMutationError::PermissionDenied => io::ErrorKind::PermissionDenied,
            StartupMutationError::Conflict(_) | StartupMutationError::InvalidState(_) => {
                io::ErrorKind::InvalidInput
            }
            StartupMutationError::IoFailure(kind) => kind,
        };
        io::Error::new(kind, err.to_string())
    }
}

/// Minimal testable registry abstraction covering only the `DeskPilot` startup value.
pub trait StartupRegistry {
    /// Reads the `DeskPilot` value. Must return `Ok(None)` when either the `Run` subkey or
    /// the `DeskPilot` value does not exist.
    fn read_value(&self) -> io::Result<Option<StartupRegistryValue>>;

    /// Writes `command` as `REG_SZ` under the `DeskPilot` value.
    fn write_value(&self, command: &str) -> io::Result<()>;

    /// Deletes only the `DeskPilot` value (never the parent `Run` subkey).
    /// Must return `Ok(())` when the key or value is already absent.
    fn delete_value(&self) -> io::Result<()>;
}

/// Parses a stored startup command expecting `"<absolute_exe_path>" <expected_arg>`.
///
/// Handles quoted paths with spaces, accepts unambiguous spaceless unquoted paths, rejects
/// unclosed/empty quotes, ambiguous unquoted paths with spaces, unexpected or missing arguments,
/// relative paths, UNC paths, and device namespace paths. Never executes or shell-expands strings.
pub fn parse_startup_command(raw: &str, expected_arg: &str) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || expected_arg.trim().is_empty() {
        return None;
    }

    let (exe_part, rest) = if let Some(after_open) = trimmed.strip_prefix('"') {
        let close_idx = after_open.find('"')?;
        let exe_part = &after_open[..close_idx];
        let after_close = &after_open[close_idx + 1..];
        if after_close.contains('"') {
            return None;
        }
        if !after_close.is_empty() && !after_close.chars().next()?.is_whitespace() {
            return None;
        }
        (exe_part, after_close.trim())
    } else {
        if trimmed.contains('"') {
            return None;
        }
        let ws_idx = trimmed.find(char::is_whitespace)?;
        (&trimmed[..ws_idx], trimmed[ws_idx..].trim())
    };

    let mut args = rest.split_whitespace();
    if args.next() != Some(expected_arg) || args.next().is_some() {
        return None;
    }

    validate_local_drive_exe_path(exe_part)
}

fn validate_local_drive_exe_path(raw: &str) -> Option<PathBuf> {
    if raw.is_empty() || raw != raw.trim() || raw.contains('\0') {
        return None;
    }
    ApplicationLauncherService::validate_lexical_path(raw).ok()?;
    let normalized = ApplicationLauncherService::normalize_verbatim_path(PathBuf::from(raw));
    let s = normalized.to_str()?;
    let bytes = s.as_bytes();
    if bytes.len() < 4
        || !bytes[0].is_ascii_alphabetic()
        || bytes[1] != b':'
        || (bytes[2] != b'\\' && bytes[2] != b'/')
    {
        return None;
    }
    if s.split(['\\', '/']).any(|seg| seg == ".." || seg == ".") {
        return None;
    }
    if !normalized.components().next().is_some_and(|c| {
        matches!(
            c,
            Component::Prefix(_) | Component::RootDir | Component::Normal(_)
        )
    }) {
        return None;
    }
    Some(PathBuf::from(raw))
}

/// Normalizes a local Windows executable path for case-insensitive and separator-insensitive
/// comparison without requiring the path to exist on disk.
pub fn normalize_windows_path(path: &Path) -> Option<String> {
    let stripped = ApplicationLauncherService::normalize_verbatim_path(path.to_path_buf());
    let raw = stripped.to_str()?.trim();
    if raw.is_empty() {
        return None;
    }
    let replaced = raw.replace('/', "\\");
    let mut out = String::with_capacity(replaced.len());
    let mut prev_sep = false;
    for ch in replaced.chars() {
        if ch == '\\' {
            if !prev_sep {
                out.push('\\');
            }
            prev_sep = true;
        } else {
            out.push(ch.to_ascii_lowercase());
            prev_sep = false;
        }
    }
    if out.len() > 3 {
        while out.ends_with('\\') {
            out.pop();
        }
    }
    Some(out)
}

fn file_basename_ascii_lower(path: &Path) -> Option<String> {
    let raw = path.to_str()?.trim();
    let base = raw.rsplit(['\\', '/']).find(|part| !part.is_empty())?;
    Some(base.to_ascii_lowercase())
}

fn paths_match(parsed_exe: &Path, current_exe: &Path, path_exists: &dyn Fn(&Path) -> bool) -> bool {
    if normalize_windows_path(parsed_exe)
        .zip(normalize_windows_path(current_exe))
        .is_some_and(|(a, b)| a == b)
    {
        return true;
    }
    if path_exists(parsed_exe)
        && path_exists(current_exe)
        && let (Ok(canon_parsed), Ok(canon_current)) = (
            std::fs::canonicalize(parsed_exe),
            std::fs::canonicalize(current_exe),
        )
    {
        return normalize_windows_path(&canon_parsed)
            .zip(normalize_windows_path(&canon_current))
            .is_some_and(|(a, b)| a == b);
    }
    false
}

/// Classifies an already-read registry value against `current_exe` and `expected_arg`.
pub fn classify_startup_value(
    raw_value: Option<&StartupRegistryValue>,
    current_exe: &Path,
    expected_arg: &str,
    path_exists: &dyn Fn(&Path) -> bool,
) -> StartupState {
    let Some(value) = raw_value else {
        return StartupState::Disabled;
    };
    let StartupRegistryValue::String(raw_command) = value else {
        return StartupState::Unrecognized;
    };
    let Some(parsed_exe) = parse_startup_command(raw_command, expected_arg) else {
        return StartupState::Unrecognized;
    };
    if paths_match(&parsed_exe, current_exe, path_exists) {
        return StartupState::Enabled;
    }
    let expected_name = file_basename_ascii_lower(current_exe);
    let parsed_name = file_basename_ascii_lower(&parsed_exe);
    if expected_name.is_none() || parsed_name != expected_name {
        return StartupState::Unrecognized;
    }
    if path_exists(&parsed_exe) {
        StartupState::OtherInstall
    } else {
        StartupState::Stale
    }
}

fn validated_current_exe_string(current_exe: &Path) -> Result<PathBuf, StartupMutationError> {
    let raw = current_exe
        .to_str()
        .ok_or(StartupMutationError::InvalidState(
            "Current executable path is not valid UTF-8.",
        ))?;
    validate_local_drive_exe_path(raw).ok_or(StartupMutationError::InvalidState(
        "Current executable path is not a supported local executable.",
    ))?;
    Ok(ApplicationLauncherService::normalize_verbatim_path(
        current_exe.to_path_buf(),
    ))
}

/// Reads and classifies DeskPilot's startup registration from `registry`.
pub fn inspect_startup_state(
    registry: &dyn StartupRegistry,
    current_exe: &Path,
    expected_arg: &str,
    path_exists: &dyn Fn(&Path) -> bool,
) -> Result<StartupState, StartupMutationError> {
    let normalized_current = validated_current_exe_string(current_exe)?;
    let raw = registry.read_value()?;
    Ok(classify_startup_value(
        raw.as_ref(),
        &normalized_current,
        expected_arg,
        path_exists,
    ))
}

/// Enables startup idempotently when absent; refuses to overwrite stale, alternate-install,
/// or unrecognized values.
pub fn enable_startup_guarded(
    registry: &dyn StartupRegistry,
    current_exe: &Path,
    expected_arg: &str,
    path_exists: &dyn Fn(&Path) -> bool,
) -> Result<(), StartupMutationError> {
    let normalized_current = validated_current_exe_string(current_exe)?;
    match inspect_startup_state(registry, &normalized_current, expected_arg, path_exists)? {
        StartupState::Disabled => {
            let cmd = startup_command(&normalized_current, expected_arg);
            registry
                .write_value(&cmd)
                .map_err(StartupMutationError::from)
        }
        StartupState::Enabled => Ok(()),
        other => Err(StartupMutationError::Conflict(other)),
    }
}

/// Disables startup idempotently when absent or owned by `current_exe`; refuses to delete
/// stale, alternate-install, or unrecognized values.
pub fn disable_startup_guarded(
    registry: &dyn StartupRegistry,
    current_exe: &Path,
    expected_arg: &str,
    path_exists: &dyn Fn(&Path) -> bool,
) -> Result<(), StartupMutationError> {
    let normalized_current = validated_current_exe_string(current_exe)?;
    match inspect_startup_state(registry, &normalized_current, expected_arg, path_exists)? {
        StartupState::Enabled => registry.delete_value().map_err(StartupMutationError::from),
        StartupState::Disabled => Ok(()),
        other => Err(StartupMutationError::Conflict(other)),
    }
}

/// Applies an explicit resolution action to a `Stale` or `OtherInstall` DeskPilot entry.
/// Refuses to mutate `Unrecognized` entries.
pub fn resolve_startup_guarded(
    registry: &dyn StartupRegistry,
    current_exe: &Path,
    expected_arg: &str,
    path_exists: &dyn Fn(&Path) -> bool,
    action: StartupResolutionAction,
) -> Result<(), StartupMutationError> {
    let normalized_current = validated_current_exe_string(current_exe)?;
    let state = inspect_startup_state(registry, &normalized_current, expected_arg, path_exists)?;
    match (action, state) {
        (StartupResolutionAction::UpdateStale, StartupState::Stale)
        | (
            StartupResolutionAction::SwitchToCurrent,
            StartupState::OtherInstall | StartupState::Stale,
        ) => {
            let cmd = startup_command(&normalized_current, expected_arg);
            registry
                .write_value(&cmd)
                .map_err(StartupMutationError::from)
        }
        (StartupResolutionAction::ClearStale, StartupState::Stale) => {
            registry.delete_value().map_err(StartupMutationError::from)
        }
        (
            StartupResolutionAction::UpdateStale | StartupResolutionAction::SwitchToCurrent,
            StartupState::Enabled,
        )
        | (StartupResolutionAction::ClearStale, StartupState::Disabled) => Ok(()),
        (_, other) => Err(StartupMutationError::Conflict(other)),
    }
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

    #[derive(Debug, Default)]
    struct FakeRegistryState {
        subkey_exists: bool,
        value: Option<StartupRegistryValue>,
        read_error: Option<io::ErrorKind>,
        write_error: Option<io::ErrorKind>,
        delete_error: Option<io::ErrorKind>,
        writes: usize,
        deletes: usize,
    }

    #[derive(Debug, Default)]
    struct FakeStartupRegistry {
        inner: Mutex<FakeRegistryState>,
    }

    impl FakeStartupRegistry {
        fn with_missing_key() -> Self {
            Self::default()
        }

        fn with_empty_key() -> Self {
            Self {
                inner: Mutex::new(FakeRegistryState {
                    subkey_exists: true,
                    ..Default::default()
                }),
            }
        }

        fn with_str_value(val: &str) -> Self {
            Self {
                inner: Mutex::new(FakeRegistryState {
                    subkey_exists: true,
                    value: Some(StartupRegistryValue::String(val.to_string())),
                    ..Default::default()
                }),
            }
        }

        fn with_unsupported_type() -> Self {
            Self {
                inner: Mutex::new(FakeRegistryState {
                    subkey_exists: true,
                    value: Some(StartupRegistryValue::UnsupportedType),
                    ..Default::default()
                }),
            }
        }

        fn set_read_error(&self, kind: io::ErrorKind) {
            self.inner.lock().unwrap().read_error = Some(kind);
        }

        fn set_write_error(&self, kind: io::ErrorKind) {
            self.inner.lock().unwrap().write_error = Some(kind);
        }

        fn set_delete_error(&self, kind: io::ErrorKind) {
            self.inner.lock().unwrap().delete_error = Some(kind);
        }

        fn snapshot(&self) -> (bool, Option<StartupRegistryValue>, usize, usize) {
            let g = self.inner.lock().unwrap();
            (g.subkey_exists, g.value.clone(), g.writes, g.deletes)
        }
    }

    impl StartupRegistry for FakeStartupRegistry {
        fn read_value(&self) -> io::Result<Option<StartupRegistryValue>> {
            let g = self.inner.lock().unwrap();
            if let Some(kind) = g.read_error {
                return Err(io::Error::new(kind, "fake read error"));
            }
            if !g.subkey_exists {
                return Ok(None);
            }
            Ok(g.value.clone())
        }

        fn write_value(&self, command: &str) -> io::Result<()> {
            let mut g = self.inner.lock().unwrap();
            if let Some(kind) = g.write_error {
                return Err(io::Error::new(kind, "fake write error"));
            }
            g.subkey_exists = true;
            g.value = Some(StartupRegistryValue::String(command.to_string()));
            g.writes += 1;
            Ok(())
        }

        fn delete_value(&self) -> io::Result<()> {
            let mut g = self.inner.lock().unwrap();
            if let Some(kind) = g.delete_error {
                return Err(io::Error::new(kind, "fake delete error"));
            }
            if !g.subkey_exists {
                return Ok(());
            }
            g.value = None;
            g.deletes += 1;
            Ok(())
        }
    }

    fn current_exe() -> &'static Path {
        Path::new(r"C:\Program Files\DeskPilot\deskpilot.exe")
    }

    #[test]
    fn missing_run_key_and_empty_key_and_missing_value_report_disabled() {
        let missing_key = FakeStartupRegistry::with_missing_key();
        assert_eq!(
            inspect_startup_state(&missing_key, current_exe(), "--autostart", &|_| false).unwrap(),
            StartupState::Disabled
        );

        let empty_key = FakeStartupRegistry::with_empty_key();
        assert_eq!(
            inspect_startup_state(&empty_key, current_exe(), "--autostart", &|_| false).unwrap(),
            StartupState::Disabled
        );

        assert_eq!(
            classify_startup_value(None, current_exe(), "--autostart", &|_| false),
            StartupState::Disabled
        );
    }

    #[test]
    fn recognizes_valid_current_executable_with_case_and_separator_differences() {
        for cmd in [
            r#""C:\Program Files\DeskPilot\deskpilot.exe" --autostart"#,
            r#""c:\program files\deskpilot\deskpilot.exe" --autostart"#,
            r#""C:/Program Files/DeskPilot/deskpilot.exe" --autostart"#,
            r#""\\?\C:\PROGRAM FILES\DeskPilot\\deskpilot.exe" --autostart"#,
        ] {
            let reg = FakeStartupRegistry::with_str_value(cmd);
            assert_eq!(
                inspect_startup_state(&reg, current_exe(), "--autostart", &|_| false).unwrap(),
                StartupState::Enabled,
                "expected Enabled for {cmd}"
            );
        }
    }

    #[test]
    fn classifies_stale_and_alternate_installs_distinctly() {
        let stale_reg = FakeStartupRegistry::with_str_value(
            r#""C:\Legacy Dir\DeskPilot\deskpilot.exe" --autostart"#,
        );
        assert_eq!(
            inspect_startup_state(&stale_reg, current_exe(), "--autostart", &|_| false).unwrap(),
            StartupState::Stale
        );

        let alt_reg = FakeStartupRegistry::with_str_value(
            r#""D:\Portable\DeskPilot\deskpilot.exe" --autostart"#,
        );
        assert_eq!(
            inspect_startup_state(&alt_reg, current_exe(), "--autostart", &|p| {
                p == Path::new(r"D:\Portable\DeskPilot\deskpilot.exe")
            })
            .unwrap(),
            StartupState::OtherInstall
        );
    }

    #[test]
    fn rejects_foreign_malformed_ambiguous_extra_args_unc_device_and_non_string() {
        let cases = [
            // Foreign executables
            r#""C:\Tools\other.exe" --autostart"#,
            r#""C:\Windows\System32\notepad.exe" --autostart"#,
            // Malformed quoted commands
            r#""C:\Program Files\DeskPilot\deskpilot.exe --autostart"#,
            r#""" --autostart"#,
            r#""   " --autostart"#,
            r#""C:\Program Files\DeskPilot\deskpilot.exe"--autostart"#,
            r#"" C:\Program Files\DeskPilot\deskpilot.exe " --autostart"#,
            r#""C:\DeskPilot\deskpilot.exe" "extra" --autostart"#,
            "",
            "   ",
            // Ambiguous unquoted paths with spaces
            r#"C:\Program Files\DeskPilot\deskpilot.exe --autostart"#,
            // Missing or unexpected arguments
            r#""C:\Program Files\DeskPilot\deskpilot.exe""#,
            r#""C:\Program Files\DeskPilot\deskpilot.exe" --autostart --debug"#,
            r#""C:\Program Files\DeskPilot\deskpilot.exe" --minimized"#,
            r#"cmd.exe /c "C:\Program Files\DeskPilot\deskpilot.exe" --autostart"#,
            // Relative or traversal paths
            r#""deskpilot.exe" --autostart"#,
            r#""C:\Program Files\DeskPilot\..\Other\deskpilot.exe" --autostart"#,
            // UNC paths
            r#""\\server\share\deskpilot.exe" --autostart"#,
            r#""//server/share/deskpilot.exe" --autostart"#,
            r#""\\?\UNC\server\share\deskpilot.exe" --autostart"#,
            // Device namespace paths
            r#""\\.\C:\Program Files\DeskPilot\deskpilot.exe" --autostart"#,
            r#""\\?\GlobalRoot\Device\HarddiskVolume1\deskpilot.exe" --autostart"#,
            r#""\\?\Volume{12345678-1234-1234-1234-123456789012}\deskpilot.exe" --autostart"#,
            r#""\\?\PIPE\deskpilot.exe" --autostart"#,
        ];

        for raw in cases {
            let reg = FakeStartupRegistry::with_str_value(raw);
            assert_eq!(
                inspect_startup_state(&reg, current_exe(), "--autostart", &|_| true).unwrap(),
                StartupState::Unrecognized,
                "expected Unrecognized for {raw:?}"
            );
        }

        let non_str = FakeStartupRegistry::with_unsupported_type();
        assert_eq!(
            inspect_startup_state(&non_str, current_exe(), "--autostart", &|_| true).unwrap(),
            StartupState::Unrecognized
        );
    }

    #[test]
    fn permission_denied_is_preserved_without_leaking_paths() {
        let read_denied = FakeStartupRegistry::with_empty_key();
        read_denied.set_read_error(io::ErrorKind::PermissionDenied);
        let err = inspect_startup_state(&read_denied, current_exe(), "--autostart", &|_| false)
            .unwrap_err();
        assert_eq!(err, StartupMutationError::PermissionDenied);
        assert!(!err.to_string().contains("CurrentVersion"));
        assert!(!err.to_string().contains("Program Files"));
        assert_eq!(io::Error::from(err).kind(), io::ErrorKind::PermissionDenied);

        let write_denied = FakeStartupRegistry::with_empty_key();
        write_denied.set_write_error(io::ErrorKind::PermissionDenied);
        let err = enable_startup_guarded(&write_denied, current_exe(), "--autostart", &|_| false)
            .unwrap_err();
        assert_eq!(err, StartupMutationError::PermissionDenied);

        let delete_denied = FakeStartupRegistry::with_str_value(
            r#""C:\Program Files\DeskPilot\deskpilot.exe" --autostart"#,
        );
        delete_denied.set_delete_error(io::ErrorKind::PermissionDenied);
        let err = disable_startup_guarded(&delete_denied, current_exe(), "--autostart", &|_| false)
            .unwrap_err();
        assert_eq!(err, StartupMutationError::PermissionDenied);
    }

    #[test]
    fn enable_and_disable_are_idempotent() {
        let reg = FakeStartupRegistry::with_missing_key();
        enable_startup_guarded(&reg, current_exe(), "--autostart", &|_| false).unwrap();
        let (subkey_exists, value, writes, deletes) = reg.snapshot();
        assert!(subkey_exists);
        assert_eq!(
            value,
            Some(StartupRegistryValue::String(
                r#""C:\Program Files\DeskPilot\deskpilot.exe" --autostart"#.to_string()
            ))
        );
        assert_eq!((writes, deletes), (1, 0));

        // Second enable is a no-op.
        enable_startup_guarded(&reg, current_exe(), "--autostart", &|_| false).unwrap();
        assert_eq!(reg.snapshot().2, 1);

        // Disable removes the value cleanly; second disable is a no-op.
        disable_startup_guarded(&reg, current_exe(), "--autostart", &|_| false).unwrap();
        let (subkey_exists, value, writes, deletes) = reg.snapshot();
        assert!(subkey_exists);
        assert_eq!(value, None);
        assert_eq!((writes, deletes), (1, 1));

        disable_startup_guarded(&reg, current_exe(), "--autostart", &|_| false).unwrap();
        assert_eq!(reg.snapshot().3, 1);
    }

    #[test]
    fn refuses_to_overwrite_or_delete_stale_other_install_or_foreign_entries() {
        let protected_cases = [
            (
                r#""C:\MissingOld\deskpilot.exe" --autostart"#,
                false,
                StartupState::Stale,
            ),
            (
                r#""D:\ExistingOther\deskpilot.exe" --autostart"#,
                true,
                StartupState::OtherInstall,
            ),
            (
                r#""C:\ThirdParty\other.exe" --autostart"#,
                true,
                StartupState::Unrecognized,
            ),
        ];

        for (raw, exists_flag, expected_state) in protected_cases {
            let reg = FakeStartupRegistry::with_str_value(raw);
            let exists = |_: &Path| exists_flag;

            let enable_err =
                enable_startup_guarded(&reg, current_exe(), "--autostart", &exists).unwrap_err();
            assert_eq!(enable_err, StartupMutationError::Conflict(expected_state));

            let disable_err =
                disable_startup_guarded(&reg, current_exe(), "--autostart", &exists).unwrap_err();
            assert_eq!(disable_err, StartupMutationError::Conflict(expected_state));

            let (_, value, writes, deletes) = reg.snapshot();
            assert_eq!(value, Some(StartupRegistryValue::String(raw.to_string())));
            assert_eq!((writes, deletes), (0, 0));
        }

        let non_str = FakeStartupRegistry::with_unsupported_type();
        assert_eq!(
            enable_startup_guarded(&non_str, current_exe(), "--autostart", &|_| false).unwrap_err(),
            StartupMutationError::Conflict(StartupState::Unrecognized)
        );
        assert_eq!(
            disable_startup_guarded(&non_str, current_exe(), "--autostart", &|_| false)
                .unwrap_err(),
            StartupMutationError::Conflict(StartupState::Unrecognized)
        );
        assert_eq!(
            non_str.snapshot().1,
            Some(StartupRegistryValue::UnsupportedType)
        );
    }

    #[test]
    fn resolve_actions_support_stale_and_other_install_and_refuse_unrecognized() {
        // 1. UpdateStale replaces missing-target DeskPilot entry.
        let stale =
            FakeStartupRegistry::with_str_value(r#""C:\MissingOld\deskpilot.exe" --autostart"#);
        resolve_startup_guarded(
            &stale,
            current_exe(),
            "--autostart",
            &|_| false,
            StartupResolutionAction::UpdateStale,
        )
        .unwrap();
        assert_eq!(
            inspect_startup_state(&stale, current_exe(), "--autostart", &|_| false).unwrap(),
            StartupState::Enabled
        );

        // 2. ClearStale deletes missing-target DeskPilot entry, refuses OtherInstall.
        let stale2 =
            FakeStartupRegistry::with_str_value(r#""C:\MissingOld\deskpilot.exe" --autostart"#);
        resolve_startup_guarded(
            &stale2,
            current_exe(),
            "--autostart",
            &|_| false,
            StartupResolutionAction::ClearStale,
        )
        .unwrap();
        assert_eq!(stale2.snapshot().1, None);

        let other =
            FakeStartupRegistry::with_str_value(r#""D:\ExistingOther\deskpilot.exe" --autostart"#);
        assert_eq!(
            resolve_startup_guarded(
                &other,
                current_exe(),
                "--autostart",
                &|_| true,
                StartupResolutionAction::UpdateStale,
            )
            .unwrap_err(),
            StartupMutationError::Conflict(StartupState::OtherInstall)
        );
        resolve_startup_guarded(
            &other,
            current_exe(),
            "--autostart",
            &|_| true,
            StartupResolutionAction::SwitchToCurrent,
        )
        .unwrap();
        assert_eq!(
            inspect_startup_state(&other, current_exe(), "--autostart", &|_| true).unwrap(),
            StartupState::Enabled
        );

        // 3. All resolve actions strictly refuse Unrecognized entries.
        let foreign = FakeStartupRegistry::with_str_value(r#""C:\Other\app.exe" --autostart"#);
        for action in [
            StartupResolutionAction::UpdateStale,
            StartupResolutionAction::ClearStale,
            StartupResolutionAction::SwitchToCurrent,
        ] {
            assert_eq!(
                resolve_startup_guarded(&foreign, current_exe(), "--autostart", &|_| true, action)
                    .unwrap_err(),
                StartupMutationError::Conflict(StartupState::Unrecognized)
            );
        }
    }
}
