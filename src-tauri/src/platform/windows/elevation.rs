//! Detects whether DeskPilot runs with an elevated (administrator) token.
//!
//! Child processes inherit the parent's token, so every app launched from an elevated
//! DeskPilot also runs as administrator. The UI uses this to warn the user.

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::Security::{
    GetTokenInformation, TOKEN_ELEVATION, TOKEN_QUERY, TokenElevation,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

/// Returns `Some(true)` when the current process token is elevated, `Some(false)` when it
/// is not, and `None` when Windows does not answer (the caller then shows no warning).
pub fn is_process_elevated() -> Option<bool> {
    let mut token: HANDLE = std::ptr::null_mut();
    // SAFETY: GetCurrentProcess returns a pseudo-handle that needs no closing; `token` is
    // a valid out-pointer for the opened token handle.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) } == 0 {
        return None;
    }

    let mut elevation = TOKEN_ELEVATION { TokenIsElevated: 0 };
    let mut returned: u32 = 0;
    // SAFETY: `token` was opened above with TOKEN_QUERY; the buffer is a TOKEN_ELEVATION
    // and its exact size is passed.
    let ok = unsafe {
        GetTokenInformation(
            token,
            TokenElevation,
            (&mut elevation as *mut TOKEN_ELEVATION).cast(),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut returned,
        )
    };
    // SAFETY: `token` is a handle this function owns and closes exactly once.
    unsafe { CloseHandle(token) };

    (ok != 0).then_some(elevation.TokenIsElevated != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elevation_query_answers() {
        // Whether the test runner is elevated depends on the machine; the query itself
        // must succeed for the current process.
        assert!(is_process_elevated().is_some());
    }
}
