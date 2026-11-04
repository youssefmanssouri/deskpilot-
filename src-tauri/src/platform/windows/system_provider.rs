use crate::error::DomainError;
use crate::models::system::{DriveMetrics, MemoryMetrics, OsInfoMetrics, SystemTimesSample};
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::time::{SystemTime, UNIX_EPOCH};
use windows_sys::Win32::Foundation::FILETIME;
use windows_sys::Win32::Storage::FileSystem::{
    GetDiskFreeSpaceExW, GetDriveTypeW, GetLogicalDriveStringsW, GetVolumeInformationW,
};

/// Win32 drive type constant for fixed local disks (hard drives, SSDs).
const DRIVE_FIXED: u32 = 3;
use windows_sys::Win32::System::SystemInformation::{
    GetNativeSystemInfo, GetTickCount64, GlobalMemoryStatusEx, MEMORYSTATUSEX,
    PROCESSOR_ARCHITECTURE_AMD64, PROCESSOR_ARCHITECTURE_ARM, PROCESSOR_ARCHITECTURE_ARM64,
    PROCESSOR_ARCHITECTURE_INTEL, SYSTEM_INFO,
};
use windows_sys::Win32::System::Threading::GetSystemTimes;
use winreg::RegKey;
use winreg::enums::HKEY_LOCAL_MACHINE;

/// Native Windows platform provider for System Intelligence metrics.
///
/// Implements pure, sub-microsecond Win32 API calls (`kernel32.dll`) and read-only
/// registry lookups. Strictly excludes WMI, PowerShell, shell spawning, and elevated privileges.
pub struct WindowsSystemProvider;

impl WindowsSystemProvider {
    /// Queries cumulative system CPU times across all processors via `GetSystemTimes`.
    ///
    /// Returns 100-nanosecond tick counts for idle, kernel, and user times along with
    /// the current Unix epoch millisecond timestamp.
    pub fn query_system_times() -> Result<SystemTimesSample, DomainError> {
        let mut idle_ft = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let mut kernel_ft = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let mut user_ft = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };

        let success = unsafe {
            GetSystemTimes(
                &mut idle_ft as *mut FILETIME,
                &mut kernel_ft as *mut FILETIME,
                &mut user_ft as *mut FILETIME,
            )
        };

        if success == 0 {
            return Err(DomainError::Unavailable(
                "Failed to query system times via GetSystemTimes".to_string(),
            ));
        }

        let to_u64 = |ft: FILETIME| ((ft.dwHighDateTime as u64) << 32) | (ft.dwLowDateTime as u64);

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Ok(SystemTimesSample::new(
            to_u64(idle_ft),
            to_u64(kernel_ft),
            to_u64(user_ft),
            now_ms,
        ))
    }

    /// Queries CPU architecture, logical core count, and marketing brand name.
    ///
    /// Hardware architecture and core counts are retrieved via `GetNativeSystemInfo`.
    /// Processor model string is read from the non-privileged `HKLM\HARDWARE\DESCRIPTION\System\CentralProcessor\0`.
    pub fn query_cpu_info() -> Result<(String, u32, String), DomainError> {
        let mut sys_info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
        unsafe {
            GetNativeSystemInfo(&mut sys_info as *mut SYSTEM_INFO);
        }

        let logical_cores = sys_info.dwNumberOfProcessors;
        let arch_raw = unsafe { sys_info.Anonymous.Anonymous.wProcessorArchitecture };

        let architecture = match arch_raw {
            PROCESSOR_ARCHITECTURE_AMD64 => "x86_64".to_string(),
            PROCESSOR_ARCHITECTURE_ARM64 => "ARM64".to_string(),
            PROCESSOR_ARCHITECTURE_INTEL => "x86".to_string(),
            PROCESSOR_ARCHITECTURE_ARM => "ARM".to_string(),
            _ => "Unknown".to_string(),
        };

        // Query processor brand string from Windows registry (read-only, non-elevated)
        let brand_name = RegKey::predef(HKEY_LOCAL_MACHINE)
            .open_subkey("HARDWARE\\DESCRIPTION\\System\\CentralProcessor\\0")
            .and_then(|key| key.get_value::<String, _>("ProcessorNameString"))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| format!("{} Compatible Processor", architecture));

        Ok((brand_name, logical_cores, architecture))
    }

    /// Queries physical RAM capacity, available memory, and usage via `GlobalMemoryStatusEx`.
    pub fn query_memory() -> Result<MemoryMetrics, DomainError> {
        let mut mem_status: MEMORYSTATUSEX = unsafe { std::mem::zeroed() };
        mem_status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;

        let success = unsafe { GlobalMemoryStatusEx(&mut mem_status as *mut MEMORYSTATUSEX) };
        if success == 0 {
            return Err(DomainError::Unavailable(
                "Failed to query system memory via GlobalMemoryStatusEx".to_string(),
            ));
        }

        let total_bytes = mem_status.ullTotalPhys;
        let available_bytes = mem_status.ullAvailPhys;
        let used_bytes = total_bytes.saturating_sub(available_bytes);
        let used_percent = if total_bytes > 0 {
            ((used_bytes as f64 / total_bytes as f64) * 1000.0).round() / 10.0
        } else {
            0.0
        };

        Ok(MemoryMetrics {
            total_bytes,
            available_bytes,
            used_bytes,
            used_percent,
        })
    }

    /// Enumerates active fixed local storage volumes and queries capacity and labels.
    ///
    /// Network shares, optical drives, and non-fixed media are filtered out to prevent
    /// I/O timeouts and network stalls.
    pub fn query_drives() -> Result<Vec<DriveMetrics>, DomainError> {
        // Enumerate drive strings: GetLogicalDriveStringsW
        let mut buffer: [u16; 512] = [0; 512];
        let chars_written =
            unsafe { GetLogicalDriveStringsW(buffer.len() as u32, buffer.as_mut_ptr()) };

        if chars_written == 0 || chars_written as usize >= buffer.len() {
            return Err(DomainError::Unavailable(
                "Failed to enumerate drive strings via GetLogicalDriveStringsW".to_string(),
            ));
        }

        // Buffer contains null-separated strings terminated by double null (e.g. "C:\\\0D:\\\0\0")
        let mut drives = Vec::new();
        let mut current_offset = 0;

        while current_offset < chars_written as usize {
            let slice = &buffer[current_offset..];
            let len = slice.iter().position(|&c| c == 0).unwrap_or(0);
            if len == 0 {
                break;
            }

            let drive_wide = &slice[..len];
            let root_path = OsString::from_wide(drive_wide)
                .to_string_lossy()
                .to_string();

            // Null-terminate for Win32 API calls
            let mut root_wide_null: Vec<u16> = drive_wide.to_vec();
            root_wide_null.push(0);

            let drive_type = unsafe { GetDriveTypeW(root_wide_null.as_ptr()) };

            // We strictly target DRIVE_FIXED local partitions (e.g. C:\, D:\)
            if drive_type == DRIVE_FIXED {
                let mut volume_name_buf: [u16; 261] = [0; 261];
                let mut fs_name_buf: [u16; 261] = [0; 261];

                // lpVolumeSerialNumber is explicitly NULL to enforce privacy
                let vol_success = unsafe {
                    GetVolumeInformationW(
                        root_wide_null.as_ptr(),
                        volume_name_buf.as_mut_ptr(),
                        volume_name_buf.len() as u32,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        fs_name_buf.as_mut_ptr(),
                        fs_name_buf.len() as u32,
                    )
                };

                let volume_name = if vol_success != 0 {
                    let len = volume_name_buf.iter().position(|&c| c == 0).unwrap_or(0);
                    let name = OsString::from_wide(&volume_name_buf[..len])
                        .to_string_lossy()
                        .trim()
                        .to_string();
                    if name.is_empty() {
                        "Local Disk".to_string()
                    } else {
                        name
                    }
                } else {
                    "Local Disk".to_string()
                };

                let file_system = if vol_success != 0 {
                    let len = fs_name_buf.iter().position(|&c| c == 0).unwrap_or(0);
                    OsString::from_wide(&fs_name_buf[..len])
                        .to_string_lossy()
                        .trim()
                        .to_string()
                } else {
                    "Unknown".to_string()
                };

                let mut free_available: u64 = 0;
                let mut total_bytes: u64 = 0;
                let mut total_free: u64 = 0;

                let space_success = unsafe {
                    GetDiskFreeSpaceExW(
                        root_wide_null.as_ptr(),
                        &mut free_available as *mut u64,
                        &mut total_bytes as *mut u64,
                        &mut total_free as *mut u64,
                    )
                };

                if space_success != 0 && total_bytes > 0 {
                    let available_bytes = free_available;
                    let used_bytes = total_bytes.saturating_sub(available_bytes);
                    let used_percent =
                        ((used_bytes as f64 / total_bytes as f64) * 1000.0).round() / 10.0;

                    drives.push(DriveMetrics {
                        root_path,
                        volume_name,
                        file_system,
                        total_bytes,
                        available_bytes,
                        used_bytes,
                        used_percent,
                    });
                }
            }

            current_offset += len + 1;
        }

        drives.sort_by(|a, b| a.root_path.cmp(&b.root_path));
        Ok(drives)
    }

    /// Queries monotonic system uptime in seconds since OS boot via `GetTickCount64`.
    pub fn query_uptime_seconds() -> Result<u64, DomainError> {
        let ms = unsafe { GetTickCount64() };
        Ok(ms / 1000)
    }

    /// Queries complete OS and platform runtime metadata.
    pub fn query_os_info(app_uptime_seconds: u64) -> Result<OsInfoMetrics, DomainError> {
        let uptime_seconds = Self::query_uptime_seconds()?;
        let (_, _, architecture) = Self::query_cpu_info()?;

        Ok(OsInfoMetrics {
            uptime_seconds,
            app_uptime_seconds,
            os_name: "Windows".to_string(),
            architecture,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_system_times_returns_positive_values() {
        let sample = WindowsSystemProvider::query_system_times()
            .expect("query_system_times must succeed on Windows");
        assert!(sample.idle_time > 0);
        assert!(sample.kernel_time > 0);
        assert!(sample.user_time > 0);
        assert!(sample.sampled_at_ms > 0);
        // In Windows NT, kernel time includes idle time
        assert!(sample.kernel_time >= sample.idle_time);
    }

    #[test]
    fn test_query_cpu_info_returns_valid_data() {
        let (brand, cores, arch) = WindowsSystemProvider::query_cpu_info()
            .expect("query_cpu_info must succeed on Windows");
        assert!(!brand.is_empty());
        assert!(cores > 0);
        assert_eq!(arch, "x86_64");
    }

    #[test]
    fn test_query_memory_returns_positive_ram() {
        let mem =
            WindowsSystemProvider::query_memory().expect("query_memory must succeed on Windows");
        assert!(mem.total_bytes > 0);
        assert!(mem.available_bytes > 0);
        assert!(mem.total_bytes >= mem.available_bytes);
        assert!(mem.used_percent >= 0.0 && mem.used_percent <= 100.0);
    }

    #[test]
    fn test_query_drives_enumerates_fixed_storage() {
        let drives =
            WindowsSystemProvider::query_drives().expect("query_drives must succeed on Windows");
        assert!(!drives.is_empty());
        let c_drive = drives.iter().find(|d| d.root_path.starts_with("C:"));
        assert!(c_drive.is_some(), "C: drive must be present");
        let c = c_drive.unwrap();
        assert!(c.total_bytes > 0);
        assert!(!c.volume_name.is_empty());
        assert!(!c.file_system.is_empty());
        assert!(c.used_percent >= 0.0 && c.used_percent <= 100.0);
    }

    #[test]
    fn test_query_uptime_seconds_monotonic() {
        let uptime = WindowsSystemProvider::query_uptime_seconds()
            .expect("query_uptime_seconds must succeed on Windows");
        assert!(uptime > 0);
    }

    #[test]
    fn test_query_os_info_returns_expected_platform() {
        let os_info = WindowsSystemProvider::query_os_info(42)
            .expect("query_os_info must succeed on Windows");
        assert_eq!(os_info.os_name, "Windows");
        assert_eq!(os_info.architecture, "x86_64");
        assert_eq!(os_info.app_uptime_seconds, 42);
        assert!(os_info.uptime_seconds > 0);
    }
}
