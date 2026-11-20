//! "Start with Windows": one value in the current user's Run key.
//!
//! Written directly instead of through tauri-plugin-autostart because that plugin may
//! write to the machine-wide key (HKLM) when DeskPilot runs as administrator, and writes
//! the executable path unquoted. Here the entry is always per-user (HKCU) and quoted.

use std::io;
use winreg::RegKey;
use winreg::enums::{HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

/// Name of DeskPilot's value under the Run key.
pub const STARTUP_VALUE_NAME: &str = "DeskPilot";

/// True if DeskPilot's startup value exists for the current user.
pub fn is_start_with_windows_enabled() -> io::Result<bool> {
    let key = RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(RUN_KEY, KEY_READ)?;
    match key.get_value::<String, _>(STARTUP_VALUE_NAME) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e),
    }
}

/// Adds (or updates) DeskPilot's startup value with `command`
/// (see `core::settings::startup_command`).
pub fn enable_start_with_windows(command: &str) -> io::Result<()> {
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(RUN_KEY)?;
    key.set_value(STARTUP_VALUE_NAME, &command)
}

/// Removes DeskPilot's startup value. Succeeds if it was already absent.
pub fn disable_start_with_windows() -> io::Result<()> {
    let key = match RegKey::predef(HKEY_CURRENT_USER).open_subkey_with_flags(RUN_KEY, KEY_SET_VALUE)
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
