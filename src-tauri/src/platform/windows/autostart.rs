//! "Start with Windows": one value in the current user's Run key.
//!
//! Written directly instead of through tauri-plugin-autostart because that plugin may
//! write to the machine-wide key (HKLM) when DeskPilot runs as administrator, and writes
//! the executable path unquoted. Here the entry is always per-user (HKCU) and quoted.

use crate::core::settings::{
    StartupMutationError, StartupRegistry, StartupRegistryValue, StartupState,
    disable_startup_guarded, enable_startup_guarded, inspect_startup_state, normalize_windows_path,
    parse_startup_command,
};
use std::io;
use std::path::Path;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_SZ};
use winreg::{RegKey, RegValue};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// Name of DeskPilot's value under the Run key.
pub const STARTUP_VALUE_NAME: &str = "DeskPilot";

/// Registry store targeting a single `DeskPilot` value under an `HKCU` subkey.
#[derive(Debug, Clone)]
pub struct WindowsRunKeyStore {
    subkey: String,
}

impl WindowsRunKeyStore {
    /// Targets the production per-user startup key:
    /// `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.
    pub fn current_user_run() -> Self {
        Self {
            subkey: RUN_KEY.to_string(),
        }
    }

    #[cfg(test)]
    pub(crate) fn for_test_subkey(subkey: impl Into<String>) -> Self {
        Self {
            subkey: subkey.into(),
        }
    }
}

fn decode_reg_sz_value(raw: &RegValue) -> StartupRegistryValue {
    if raw.vtype != REG_SZ || !raw.bytes.len().is_multiple_of(2) {
        return StartupRegistryValue::UnsupportedType;
    }
    let mut words: Vec<u16> = raw
        .bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|&pair| u16::from_le_bytes(pair))
        .collect();
    while words.last() == Some(&0) {
        words.pop();
    }
    match String::from_utf16(&words) {
        Ok(s) => StartupRegistryValue::String(s),
        Err(_) => StartupRegistryValue::UnsupportedType,
    }
}

impl StartupRegistry for WindowsRunKeyStore {
    fn read_value(&self) -> io::Result<Option<StartupRegistryValue>> {
        let key = match RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(&self.subkey, KEY_READ)
        {
            Ok(key) => key,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        let raw = match key.get_raw_value(STARTUP_VALUE_NAME) {
            Ok(val) => val,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(e),
        };
        Ok(Some(decode_reg_sz_value(&raw)))
    }

    fn write_value(&self, command: &str) -> io::Result<()> {
        let (key, _) = RegKey::predef(HKEY_CURRENT_USER)
            .create_subkey_with_flags(&self.subkey, KEY_SET_VALUE)?;
        key.set_value(STARTUP_VALUE_NAME, &command)
    }

    fn delete_value(&self) -> io::Result<()> {
        let key = match RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey_with_flags(&self.subkey, KEY_SET_VALUE)
        {
            Ok(key) => key,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
        };
        match key.delete_value(STARTUP_VALUE_NAME) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(e),
        }
    }
}

/// Inspects DeskPilot's startup registration state for the current user and executable.
pub fn inspect_start_with_windows_state() -> Result<StartupState, StartupMutationError> {
    let current_exe = std::env::current_exe().map_err(StartupMutationError::from)?;
    inspect_startup_state(
        &WindowsRunKeyStore::current_user_run(),
        &current_exe,
        crate::AUTOSTART_ARG,
        &|p: &Path| p.is_file(),
    )
}

/// True if DeskPilot's startup value is validly registered for the current executable.
/// Returns `Ok(false)` when the `Run` subkey or `DeskPilot` value is absent, and `Err`
/// on permission errors or unrecognized/stale entries so callers do not confuse them with `false`.
pub fn is_start_with_windows_enabled() -> io::Result<bool> {
    match inspect_start_with_windows_state().map_err(io::Error::from)? {
        StartupState::Disabled => Ok(false),
        StartupState::Enabled => Ok(true),
        StartupState::Stale | StartupState::OtherInstall | StartupState::Unrecognized => {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Existing startup entry is stale or unrecognized.",
            ))
        }
    }
}

/// Adds DeskPilot's startup value when absent, after verifying `command` matches the current
/// executable and any pre-existing value is safe. Refuses to overwrite foreign or stale values.
pub fn enable_start_with_windows(command: &str) -> io::Result<()> {
    let current_exe = std::env::current_exe()?;
    let parsed = parse_startup_command(command, crate::AUTOSTART_ARG).ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Invalid startup command format.",
        )
    })?;
    if normalize_windows_path(&parsed) != normalize_windows_path(&current_exe) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Startup command must target the running executable.",
        ));
    }
    enable_startup_guarded(
        &WindowsRunKeyStore::current_user_run(),
        &current_exe,
        crate::AUTOSTART_ARG,
        &|p: &Path| p.is_file(),
    )
    .map_err(io::Error::from)
}

/// Removes DeskPilot's startup value when absent or owned by the current executable.
/// Refuses to delete stale, alternate-install, or unrecognized entries.
pub fn disable_start_with_windows() -> io::Result<()> {
    let current_exe = std::env::current_exe()?;
    disable_startup_guarded(
        &WindowsRunKeyStore::current_user_run(),
        &current_exe,
        crate::AUTOSTART_ARG,
        &|p: &Path| p.is_file(),
    )
    .map_err(io::Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::settings::startup_command;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static TEST_COUNTER: AtomicU64 = AtomicU64::new(0);

    struct ScopedTestKey {
        subkey: String,
    }

    impl ScopedTestKey {
        fn new(name: &str) -> Self {
            let seq = TEST_COUNTER.fetch_add(1, Ordering::Relaxed);
            let subkey = format!(
                r"Software\DeskPilotTest\autostart-{}-{}-{}",
                std::process::id(),
                seq,
                name
            );
            let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(&subkey);
            Self { subkey }
        }

        fn store(&self) -> WindowsRunKeyStore {
            WindowsRunKeyStore::for_test_subkey(&self.subkey)
        }

        fn create_empty(&self) -> RegKey {
            RegKey::predef(HKEY_CURRENT_USER)
                .create_subkey(&self.subkey)
                .unwrap()
                .0
        }
    }

    impl Drop for ScopedTestKey {
        fn drop(&mut self) {
            let _ = RegKey::predef(HKEY_CURRENT_USER).delete_subkey_all(&self.subkey);
        }
    }

    fn fake_exe() -> PathBuf {
        PathBuf::from(r"C:\Program Files\DeskPilot\deskpilot.exe")
    }

    #[test]
    fn windows_missing_disposable_subkey_reports_disabled_and_enables_cleanly() {
        let guard = ScopedTestKey::new("missing");
        let store = guard.store();
        assert_eq!(
            inspect_startup_state(&store, &fake_exe(), "--autostart", &|_| false).unwrap(),
            StartupState::Disabled
        );

        // Idempotent disable on missing subkey succeeds without creating key.
        disable_startup_guarded(&store, &fake_exe(), "--autostart", &|_| false).unwrap();
        assert!(
            RegKey::predef(HKEY_CURRENT_USER)
                .open_subkey_with_flags(&guard.subkey, KEY_READ)
                .is_err()
        );

        // Enable creates subkey and writes quoted REG_SZ value.
        enable_startup_guarded(&store, &fake_exe(), "--autostart", &|_| false).unwrap();
        assert_eq!(
            inspect_startup_state(&store, &fake_exe(), "--autostart", &|_| false).unwrap(),
            StartupState::Enabled
        );
    }

    #[test]
    fn windows_empty_subkey_and_sibling_values_are_preserved() {
        let guard = ScopedTestKey::new("siblings");
        let raw_key = guard.create_empty();
        raw_key
            .set_value("OtherApp", &"\"C:\\Other\\app.exe\"")
            .unwrap();

        let store = guard.store();
        assert_eq!(
            inspect_startup_state(&store, &fake_exe(), "--autostart", &|_| false).unwrap(),
            StartupState::Disabled
        );

        enable_startup_guarded(&store, &fake_exe(), "--autostart", &|_| false).unwrap();
        disable_startup_guarded(&store, &fake_exe(), "--autostart", &|_| false).unwrap();

        assert_eq!(
            inspect_startup_state(&store, &fake_exe(), "--autostart", &|_| false).unwrap(),
            StartupState::Disabled
        );
        // Deleting DeskPilot must never delete sibling values or the subkey itself.
        let sibling: String = raw_key.get_value("OtherApp").unwrap();
        assert_eq!(sibling, "\"C:\\Other\\app.exe\"");
    }

    #[test]
    fn windows_rejects_non_reg_sz_and_invalid_utf16_and_protects_foreign_values() {
        let guard = ScopedTestKey::new("types");
        let raw_key = guard.create_empty();
        let store = guard.store();

        // 1. REG_DWORD -> Unrecognized + refused overwrite/delete
        raw_key.set_value(STARTUP_VALUE_NAME, &1u32).unwrap();
        assert_eq!(
            inspect_startup_state(&store, &fake_exe(), "--autostart", &|_| false).unwrap(),
            StartupState::Unrecognized
        );
        assert_eq!(
            enable_startup_guarded(&store, &fake_exe(), "--autostart", &|_| false).unwrap_err(),
            StartupMutationError::Conflict(StartupState::Unrecognized)
        );
        assert_eq!(
            disable_startup_guarded(&store, &fake_exe(), "--autostart", &|_| false).unwrap_err(),
            StartupMutationError::Conflict(StartupState::Unrecognized)
        );
        assert_eq!(
            raw_key.get_value::<u32, _>(STARTUP_VALUE_NAME).unwrap(),
            1u32
        );

        // 2. REG_EXPAND_SZ -> Unrecognized (must reject non-REG_SZ even if string-like)
        let expand_bytes: Vec<u8> = startup_command(&fake_exe(), "--autostart")
            .encode_utf16()
            .chain(Some(0))
            .flat_map(u16::to_le_bytes)
            .collect();
        let expand_val = RegValue {
            vtype: winreg::enums::REG_EXPAND_SZ,
            bytes: expand_bytes.into(),
        };
        raw_key
            .set_raw_value(STARTUP_VALUE_NAME, &expand_val)
            .unwrap();
        assert_eq!(
            inspect_startup_state(&store, &fake_exe(), "--autostart", &|_| false).unwrap(),
            StartupState::Unrecognized
        );

        // 3. Odd-byte / invalid UTF-16 REG_SZ -> UnsupportedType -> Unrecognized
        let bad_utf16 = RegValue {
            vtype: REG_SZ,
            bytes: vec![0x00, 0xD8, 0x00, 0x00].into(),
        };
        raw_key
            .set_raw_value(STARTUP_VALUE_NAME, &bad_utf16)
            .unwrap();
        assert_eq!(
            inspect_startup_state(&store, &fake_exe(), "--autostart", &|_| false).unwrap(),
            StartupState::Unrecognized
        );
        assert_eq!(
            raw_key
                .get_raw_value(STARTUP_VALUE_NAME)
                .unwrap()
                .bytes
                .as_ref(),
            &[0x00, 0xD8, 0x00, 0x00]
        );
    }

    #[test]
    fn windows_stale_and_other_install_protected_under_real_registry_key() {
        let guard = ScopedTestKey::new("paths");
        let raw_key = guard.create_empty();
        let store = guard.store();

        let dir =
            std::env::temp_dir().join(format!("deskpilot-autostart-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let other_exe = dir.join("deskpilot.exe");
        std::fs::write(&other_exe, b"").unwrap();

        let alt_cmd = startup_command(&other_exe, "--autostart");
        raw_key.set_value(STARTUP_VALUE_NAME, &alt_cmd).unwrap();

        // Existing alternate executable -> OtherInstall, protected from overwrite/delete.
        assert_eq!(
            inspect_startup_state(&store, &fake_exe(), "--autostart", &|p| p.is_file()).unwrap(),
            StartupState::OtherInstall
        );
        assert_eq!(
            enable_startup_guarded(&store, &fake_exe(), "--autostart", &|p| p.is_file())
                .unwrap_err(),
            StartupMutationError::Conflict(StartupState::OtherInstall)
        );
        assert_eq!(
            disable_startup_guarded(&store, &fake_exe(), "--autostart", &|p| p.is_file())
                .unwrap_err(),
            StartupMutationError::Conflict(StartupState::OtherInstall)
        );

        // Removing the file turns it into Stale, still protected from silent overwrite/delete.
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(
            inspect_startup_state(&store, &fake_exe(), "--autostart", &|p| p.is_file()).unwrap(),
            StartupState::Stale
        );
        assert_eq!(
            enable_startup_guarded(&store, &fake_exe(), "--autostart", &|p| p.is_file())
                .unwrap_err(),
            StartupMutationError::Conflict(StartupState::Stale)
        );
        assert_eq!(
            disable_startup_guarded(&store, &fake_exe(), "--autostart", &|p| p.is_file())
                .unwrap_err(),
            StartupMutationError::Conflict(StartupState::Stale)
        );
    }
}
