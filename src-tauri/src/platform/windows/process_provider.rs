use crate::error::DomainError;
use crate::models::process::{ProcessIdentity, ProcessQueryStatus, ProcessRecord};
use windows_sys::Win32::Foundation::{CloseHandle, FILETIME, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::ProcessStatus::{
    K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS_EX,
};
use windows_sys::Win32::System::RemoteDesktop::ProcessIdToSessionId;
use windows_sys::Win32::System::Threading::{
    GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, QueryFullProcessImageNameW,
};
use windows_sys::core::BOOL;

/// Win32 error code indicating end of enumeration in Toolhelp32.
const ERROR_NO_MORE_FILES: u32 = 18;
/// Win32 error code indicating access denied by Windows security policy.
const ERROR_ACCESS_DENIED: u32 = 5;
/// Win32 error code indicating an invalid parameter or dead/non-existent PID.
const ERROR_INVALID_PARAMETER: u32 = 87;

/// Private RAII process and snapshot handle guard.
///
/// Ensures every valid `HANDLE` is closed exactly once upon drop.
/// Guarantees that `CloseHandle` is never invoked on `NULL` or `INVALID_HANDLE_VALUE`.
/// Does not leak outside the provider and is never stored in `AppState` or DTOs.
struct AutoHandle(HANDLE);

impl AutoHandle {
    /// Constructs an `AutoHandle` if the provided handle is valid.
    ///
    /// Rejects `NULL` (`0 as HANDLE`) and `INVALID_HANDLE_VALUE` (`-1 as isize as HANDLE`).
    pub fn new(handle: HANDLE) -> Option<Self> {
        if handle.is_null() || handle == INVALID_HANDLE_VALUE {
            None
        } else {
            Some(Self(handle))
        }
    }

    /// Returns the raw underlying Win32 `HANDLE`.
    pub fn raw(&self) -> HANDLE {
        self.0
    }
}

impl Drop for AutoHandle {
    fn drop(&mut self) {
        if !self.0.is_null() && self.0 != INVALID_HANDLE_VALUE {
            // SAFETY: self.0 is verified valid at construction and closed exactly once here.
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}

/// Lightweight candidate process data recorded during the initial Toolhelp32 snapshot walk.
///
/// Contains strictly lightweight metadata; process handles are NEVER opened during Pass 1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RawProcessCandidate {
    pub pid: u32,
    pub parent_pid: u32,
    pub name: String,
}

/// Windows-native process provider.
///
/// Implements read-only, least-privilege process enumeration and attribute extraction.
/// Strictly requests `PROCESS_QUERY_LIMITED_INFORMATION` (0x1000) and never requests elevated
/// rights (`PROCESS_ALL_ACCESS`, `PROCESS_VM_READ`), `SeDebugPrivilege`, or process termination.
pub struct WindowsProcessProvider;

impl WindowsProcessProvider {
    /// Enumerates all processes recorded in a freshly acquired Toolhelp32 snapshot,
    /// passing each lightweight candidate directly to the provided consumer callback.
    ///
    /// Returns the exact count of all successfully observed entries.
    /// Does not allocate an unbounded intermediate `Vec`, keeping memory strictly bounded.
    ///
    /// # Error Behavior
    /// - Normal completion occurs when `Process32NextW` returns `FALSE` and `GetLastError() == ERROR_NO_MORE_FILES`.
    /// - Any other enumeration error immediately aborts the scan with a structured `DomainError::OperationFailed`.
    pub fn enumerate_snapshot_entries<F>(mut on_candidate: F) -> Result<usize, DomainError>
    where
        F: FnMut(RawProcessCandidate) -> Result<(), DomainError>,
    {
        // SAFETY: CreateToolhelp32Snapshot is a standard documented User-Mode API.
        // TH32CS_SNAPPROCESS takes a snapshot of all system processes across all sessions.
        let raw_snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
        let snapshot = AutoHandle::new(raw_snapshot).ok_or_else(|| {
            let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            DomainError::OperationFailed(format!(
                "Failed to acquire Toolhelp32 process snapshot: OS error {}",
                err
            ))
        })?;

        let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;

        let mut total_observed: usize = 0;

        // SAFETY: entry.dwSize is correctly initialized to sizeof(PROCESSENTRY32W).
        let first_ok: BOOL = unsafe { Process32FirstW(snapshot.raw(), &mut entry) };
        if first_ok == 0 {
            let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
            if err == ERROR_NO_MORE_FILES {
                return Ok(0);
            }
            return Err(DomainError::OperationFailed(format!(
                "Toolhelp32 Process32FirstW failed: OS error {}",
                err
            )));
        }

        loop {
            total_observed = total_observed.checked_add(1).ok_or_else(|| {
                DomainError::OperationFailed("Process count arithmetic overflow".to_string())
            })?;

            let name = Self::extract_string_from_u16_slice(&entry.szExeFile);
            on_candidate(RawProcessCandidate {
                pid: entry.th32ProcessID,
                parent_pid: entry.th32ParentProcessID,
                name,
            })?;

            // SAFETY: snapshot handle is valid; entry.dwSize remains correctly set.
            let next_ok: BOOL = unsafe { Process32NextW(snapshot.raw(), &mut entry) };
            if next_ok == 0 {
                let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
                if err == ERROR_NO_MORE_FILES {
                    break;
                }
                return Err(DomainError::OperationFailed(format!(
                    "Toolhelp32 enumeration interrupted unexpectedly: OS error {}",
                    err
                )));
            }
        }

        Ok(total_observed)
    }

    /// Convenience wrapper that collects all candidates into a Vec.
    /// Primarily used for integration and diagnostic tests.
    pub fn enumerate_snapshot_candidates() -> Result<(Vec<RawProcessCandidate>, usize), DomainError>
    {
        let mut candidates = Vec::new();
        let total = Self::enumerate_snapshot_entries(|c| {
            candidates.push(c);
            Ok(())
        })?;
        Ok((candidates, total))
    }

    /// Extracts detailed attributes for a retained candidate process.
    ///
    /// Respects the strict least-privilege boundary `PROCESS_QUERY_LIMITED_INFORMATION`.
    /// Does not panic on normal process exit or access denial.
    pub fn extract_candidate_detail(
        candidate: &RawProcessCandidate,
        snapshot_seq: u64,
    ) -> ProcessRecord {
        // Special case: PID 0 (System Idle Process).
        // Per Microsoft documentation, OpenProcess always fails on PID 0 with ERROR_INVALID_PARAMETER.
        // PID 0 is an active system pseudo-process and must NEVER be classified as Exited.
        if candidate.pid == 0 {
            let name = if candidate.name.is_empty() {
                "System Idle Process".to_string()
            } else {
                candidate.name.clone()
            };
            return ProcessRecord {
                id: ProcessIdentity::ephemeral(snapshot_seq, 0),
                pid: 0,
                parent_pid: candidate.parent_pid,
                name,
                executable_path: None,
                creation_time_ticks: None,
                created_at: None,
                working_set_bytes: None,
                cpu_time_ms: None,
                session_id: None,
                status: ProcessQueryStatus::HandleUnavailable,
            };
        }

        // SAFETY: OpenProcess is called with strictly PROCESS_QUERY_LIMITED_INFORMATION (0x1000).
        // Zero write or termination rights are requested.
        let raw_handle = unsafe {
            OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION,
                0, // bInheritHandle = FALSE
                candidate.pid,
            )
        };

        let handle = match AutoHandle::new(raw_handle) {
            Some(h) => h,
            None => {
                let err = unsafe { windows_sys::Win32::Foundation::GetLastError() };
                let status = if err == ERROR_INVALID_PARAMETER {
                    // For nonzero PIDs enumerated in Toolhelp, ERROR_INVALID_PARAMETER indicates
                    // the kernel process object was destroyed before the handle could be acquired.
                    ProcessQueryStatus::Exited
                } else if err == ERROR_ACCESS_DENIED {
                    // Access restricted by Windows security policy (PPL / elevated / different session)
                    ProcessQueryStatus::HandleUnavailable
                } else {
                    // Other OS restriction preventing handle opening
                    ProcessQueryStatus::HandleUnavailable
                };

                return ProcessRecord {
                    id: ProcessIdentity::ephemeral(snapshot_seq, candidate.pid),
                    pid: candidate.pid,
                    parent_pid: candidate.parent_pid,
                    name: candidate.name.clone(),
                    executable_path: None,
                    creation_time_ticks: None,
                    created_at: None,
                    working_set_bytes: None,
                    cpu_time_ms: None,
                    session_id: None,
                    status,
                };
            }
        };

        // Query 1: Executable Path
        let mut path_buf = [0u16; 1024];
        let mut path_len = path_buf.len() as u32;
        // SAFETY: handle is valid; path_buf has path_len elements.
        let path_ok = unsafe {
            QueryFullProcessImageNameW(handle.raw(), 0, path_buf.as_mut_ptr(), &mut path_len)
        };
        let executable_path = if path_ok != 0 {
            Self::clamp_and_decode_utf16_path(&path_buf, path_len)
        } else {
            None
        };

        // Query 2: Creation & CPU Times
        let mut creation: FILETIME = unsafe { std::mem::zeroed() };
        let mut exit: FILETIME = unsafe { std::mem::zeroed() };
        let mut kernel: FILETIME = unsafe { std::mem::zeroed() };
        let mut user: FILETIME = unsafe { std::mem::zeroed() };
        // SAFETY: handle is valid; pointers point to valid stack-allocated FILETIME structs.
        let times_ok = unsafe {
            GetProcessTimes(
                handle.raw(),
                &mut creation,
                &mut exit,
                &mut kernel,
                &mut user,
            )
        };

        let (creation_time_ticks, created_at, cpu_time_ms) = if times_ok != 0 {
            let ticks = ((creation.dwHighDateTime as u64) << 32) | (creation.dwLowDateTime as u64);
            // Treat raw FILETIME validity separately from Unix/ISO-8601 conversion:
            // 1. Raw creation ticks of 0 is uninitialized/invalid and rejected (set to None).
            // 2. Nonzero raw ticks is preserved in creation_time_ticks even if ISO conversion
            //    is independently unavailable (e.g. pre-Unix-epoch timestamps).
            let valid_raw_ticks = if ticks > 0 { Some(ticks) } else { None };
            let iso = valid_raw_ticks.and_then(filetime_ticks_to_iso8601);

            let kernel_ticks =
                ((kernel.dwHighDateTime as u64) << 32) | (kernel.dwLowDateTime as u64);
            let user_ticks = ((user.dwHighDateTime as u64) << 32) | (user.dwLowDateTime as u64);
            // 1 tick = 100 ns; 10,000 ticks = 1 ms
            let total_cpu_ms = (kernel_ticks.saturating_add(user_ticks)) / 10_000;
            (valid_raw_ticks, iso, Some(total_cpu_ms))
        } else {
            (None, None, None)
        };

        // Query 3: Working Set Memory
        let mut counters: PROCESS_MEMORY_COUNTERS_EX = unsafe { std::mem::zeroed() };
        counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS_EX>() as u32;
        // SAFETY: handle is valid; counters.cb is correctly initialized.
        let mem_ok = unsafe {
            K32GetProcessMemoryInfo(handle.raw(), &mut counters as *mut _ as *mut _, counters.cb)
        };
        let working_set_bytes = if mem_ok != 0 {
            Some(counters.WorkingSetSize as u64)
        } else {
            None
        };

        // Query 4: Best-effort Session ID (takes raw PID integer, not process handle)
        let mut sess_id: u32 = 0;
        // SAFETY: pointer points to valid u32 on stack.
        let sess_ok = unsafe { ProcessIdToSessionId(candidate.pid, &mut sess_id) };
        let session_id = if sess_ok != 0 { Some(sess_id) } else { None };

        // Identity Assignment:
        // Anchored identity is issued only when valid nonzero creation ticks exist.
        // Ephemeral identity is issued when creation ticks are zero, invalid, or unavailable.
        let id = ProcessIdentity::resolve(candidate.pid, creation_time_ticks, snapshot_seq);

        // Status Determination:
        // Full: Requires executable path, a valid creation timestamp (created_at), and working set memory.
        // Full status cannot coexist with a missing creation timestamp.
        // If any of these are missing (e.g. creation_time_ticks is None, created_at is None, or path is None),
        // status degrades to Partial.
        let status =
            if executable_path.is_some() && created_at.is_some() && working_set_bytes.is_some() {
                ProcessQueryStatus::Full
            } else {
                ProcessQueryStatus::Partial
            };

        // Handle is closed automatically when `handle` goes out of scope here.
        ProcessRecord {
            id,
            pid: candidate.pid,
            parent_pid: candidate.parent_pid,
            name: candidate.name.clone(),
            executable_path,
            creation_time_ticks,
            created_at,
            working_set_bytes,
            cpu_time_ms,
            session_id,
            status,
        }
    }

    /// Extracts a sanitized path string from a raw UTF-16 wide slice,
    /// defensively clamping the returned length to the allocated buffer capacity
    /// to prevent buffer over-reads.
    pub fn clamp_and_decode_utf16_path(buf: &[u16], returned_len: u32) -> Option<String> {
        if returned_len == 0 {
            return None;
        }
        let clamped = (returned_len as usize).min(buf.len());
        let path = String::from_utf16_lossy(&buf[..clamped]);
        let trimmed = path.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }

    /// Extracts a sanitized Rust String from a null-terminated UTF-16 wide slice.
    fn extract_string_from_u16_slice(slice: &[u16]) -> String {
        let len = slice.iter().position(|&c| c == 0).unwrap_or(slice.len());
        String::from_utf16_lossy(&slice[..len]).trim().to_string()
    }
}

/// Converts Windows 64-bit FILETIME ticks to an ISO-8601 UTC string (`YYYY-MM-DDTHH:MM:SS.mmmZ`).
///
/// Uses Howard Hinnant's standard civil date algorithm purely with integer arithmetic.
/// Avoids floating-point math and external crates.
///
/// Returns `None` if `ticks` is prior to the Unix epoch (1970-01-01T00:00:00Z).
pub fn filetime_ticks_to_iso8601(ticks: u64) -> Option<String> {
    // Exact number of 100-nanosecond intervals between 1601-01-01 and 1970-01-01
    const WINDOWS_TO_UNIX_TICKS: u64 = 116_444_736_000_000_000;

    if ticks < WINDOWS_TO_UNIX_TICKS {
        return None;
    }

    let unix_ticks = ticks - WINDOWS_TO_UNIX_TICKS;
    let unix_secs = unix_ticks / 10_000_000;
    let millis = (unix_ticks % 10_000_000) / 10_000;

    let secs_of_day = (unix_secs % 86400) as u32;
    let hours = secs_of_day / 3600;
    let minutes = (secs_of_day % 3600) / 60;
    let seconds = secs_of_day % 60;

    // Howard Hinnant civil date algorithm
    let days = (unix_secs / 86400) as i64 + 719468;
    let era = (if days >= 0 { days } else { days - 146096 }) / 146097;
    let doe = (days - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };

    Some(format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
        year, m, d, hours, minutes, seconds, millis
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filetime_ticks_to_iso8601_exact_unix_epoch() {
        // Exactly 1970-01-01T00:00:00.000Z
        let ticks = 116_444_736_000_000_000;
        let iso = filetime_ticks_to_iso8601(ticks).unwrap();
        assert_eq!(iso, "1970-01-01T00:00:00.000Z");
    }

    #[test]
    fn test_filetime_ticks_to_iso8601_known_date() {
        // 2026-11-03T12:34:56.789Z
        // Unix timestamp for 2026-11-03T12:34:56.789Z:
        // Days from 1970 to 2026-11-03: 20760 days
        // Secs: 20760 * 86400 + 12 * 3600 + 34 * 60 + 56 = 1793709296
        // Ticks: 116444736000000000 + 1793709296 * 10000000 + 789 * 10000
        let unix_secs: u64 = 1793709296;
        let millis: u64 = 789;
        let ticks = 116_444_736_000_000_000 + unix_secs * 10_000_000 + millis * 10_000;
        let iso = filetime_ticks_to_iso8601(ticks).unwrap();
        assert_eq!(iso, "2026-11-03T12:34:56.789Z");
    }

    #[test]
    fn test_filetime_ticks_prior_to_unix_epoch_returns_none() {
        assert!(filetime_ticks_to_iso8601(100).is_none());
        assert!(filetime_ticks_to_iso8601(116_444_735_999_999_999).is_none());
    }

    #[test]
    fn test_auto_handle_rejects_null_and_invalid() {
        assert!(AutoHandle::new(std::ptr::null_mut()).is_none());
        assert!(AutoHandle::new(INVALID_HANDLE_VALUE).is_none());
    }

    #[test]
    fn test_extract_candidate_detail_pid_zero() {
        let candidate = RawProcessCandidate {
            pid: 0,
            parent_pid: 0,
            name: "System Idle Process".to_string(),
        };
        let record = WindowsProcessProvider::extract_candidate_detail(&candidate, 42);
        assert_eq!(record.pid, 0);
        assert_eq!(record.status, ProcessQueryStatus::HandleUnavailable);
        assert_eq!(record.id, "eph:42:0");
        assert!(record.executable_path.is_none());
        assert!(record.working_set_bytes.is_none());
    }

    #[test]
    fn test_clamp_and_decode_utf16_path() {
        // Normal valid path
        let path_utf16: Vec<u16> = "C:\\Windows\\System32\\cmd.exe".encode_utf16().collect();
        let decoded = WindowsProcessProvider::clamp_and_decode_utf16_path(
            &path_utf16,
            path_utf16.len() as u32,
        );
        assert_eq!(decoded, Some("C:\\Windows\\System32\\cmd.exe".to_string()));

        // Out-of-bounds returned length clamped to buffer length without panic
        let decoded_clamped =
            WindowsProcessProvider::clamp_and_decode_utf16_path(&path_utf16, 99999);
        assert_eq!(
            decoded_clamped,
            Some("C:\\Windows\\System32\\cmd.exe".to_string())
        );

        // Zero returned length returns None
        assert_eq!(
            WindowsProcessProvider::clamp_and_decode_utf16_path(&path_utf16, 0),
            None
        );

        // Whitespace only returns None
        let spaces: Vec<u16> = "   ".encode_utf16().collect();
        assert_eq!(
            WindowsProcessProvider::clamp_and_decode_utf16_path(&spaces, spaces.len() as u32),
            None
        );
    }
}
