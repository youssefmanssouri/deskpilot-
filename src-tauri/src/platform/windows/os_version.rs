//! Windows version detection.

use winreg::RegKey;
use winreg::enums::{HKEY_LOCAL_MACHINE, KEY_READ};

/// Returns the Windows build number (e.g. 19045 for Windows 10 22H2, 22631 for Windows 11 23H2).
///
/// Read from `HKLM\SOFTWARE\Microsoft\Windows NT\CurrentVersion\CurrentBuildNumber`, because
/// `GetVersionEx` reports an older version to apps without a compatibility manifest.
/// Returns `None` if the value cannot be read or parsed; callers then treat the
/// system as not supporting Windows 11-only features.
pub fn windows_build_number() -> Option<u32> {
    let key = RegKey::predef(HKEY_LOCAL_MACHINE)
        .open_subkey_with_flags(r"SOFTWARE\Microsoft\Windows NT\CurrentVersion", KEY_READ)
        .ok()?;
    let raw: String = key.get_value("CurrentBuildNumber").ok()?;
    raw.trim().parse().ok()
}
