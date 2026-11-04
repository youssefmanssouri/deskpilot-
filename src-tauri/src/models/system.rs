/// Domain representation of system times used for CPU load calculation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemTimesSample {
    /// Total idle time in 100-nanosecond units.
    pub idle_time: u64,
    /// Total kernel time (including idle time) in 100-nanosecond units.
    pub kernel_time: u64,
    /// Total user time in 100-nanosecond units.
    pub user_time: u64,
    /// Point in time (Unix epoch milliseconds) when this sample was taken.
    pub sampled_at_ms: u64,
}

impl SystemTimesSample {
    pub fn new(idle_time: u64, kernel_time: u64, user_time: u64, sampled_at_ms: u64) -> Self {
        Self {
            idle_time,
            kernel_time,
            user_time,
            sampled_at_ms,
        }
    }
}

/// Domain metrics for system CPU.
#[derive(Debug, Clone, PartialEq)]
pub struct CpuMetrics {
    /// Friendly brand name (e.g. "Intel(R) Core(TM) i7-10750H CPU @ 2.60GHz").
    pub brand_name: String,
    /// Total logical processor count.
    pub logical_cores: u32,
    /// System architecture string (e.g. "x86_64", "arm64").
    pub architecture: String,
    /// Current calculated CPU utilization percentage (0.0 to 100.0).
    pub utilization_percent: f64,
}

/// Domain metrics for physical system memory (RAM).
#[derive(Debug, Clone, PartialEq)]
pub struct MemoryMetrics {
    /// Total physical RAM in bytes.
    pub total_bytes: u64,
    /// Available physical RAM in bytes.
    pub available_bytes: u64,
    /// In-use physical RAM in bytes (`total_bytes - available_bytes`).
    pub used_bytes: u64,
    /// Current physical memory load percentage (0.0 to 100.0).
    pub used_percent: f64,
}

/// Domain metrics for a single local storage volume.
#[derive(Debug, Clone, PartialEq)]
pub struct DriveMetrics {
    /// Drive root path (e.g. "C:\").
    pub root_path: String,
    /// User-friendly volume label (e.g. "Windows", "Data").
    pub volume_name: String,
    /// Underlying filesystem name (e.g. "NTFS", "FAT32").
    pub file_system: String,
    /// Total storage capacity in bytes.
    pub total_bytes: u64,
    /// Available storage capacity in bytes for caller.
    pub available_bytes: u64,
    /// Used storage capacity in bytes (`total_bytes - available_bytes`).
    pub used_bytes: u64,
    /// Used percentage (0.0 to 100.0).
    pub used_percent: f64,
}

/// Domain collection of active fixed storage volumes.
#[derive(Debug, Clone, PartialEq)]
pub struct StorageMetrics {
    pub drives: Vec<DriveMetrics>,
}

/// Domain metrics for operating system and application runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OsInfoMetrics {
    /// Total operating system uptime in seconds (derived from monotonic boot ticks).
    pub uptime_seconds: u64,
    /// DeskPilot application uptime in seconds.
    pub app_uptime_seconds: u64,
    /// Operating system platform descriptor (e.g. "Windows").
    pub os_name: String,
    /// Host processor architecture (e.g. "x86_64", "ARM64").
    pub architecture: String,
}

/// Comprehensive point-in-time system intelligence snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct SystemMetrics {
    pub cpu: CpuMetrics,
    pub memory: MemoryMetrics,
    pub storage: StorageMetrics,
    pub system: OsInfoMetrics,
    pub sampled_at_ms: u64,
}

/// Coordinated state for synchronized, rate-limited CPU utilization sampling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CpuCoordinatorState {
    /// Baseline system times sample from previous evaluation.
    pub baseline_sample: SystemTimesSample,
    /// Cached CPU utilization percentage computed at `last_evaluated_at_ms`.
    pub last_utilization: f64,
    /// Unix timestamp in milliseconds when `last_utilization` was evaluated.
    pub last_evaluated_at_ms: u64,
}

impl CpuCoordinatorState {
    pub fn new(
        baseline_sample: SystemTimesSample,
        last_utilization: f64,
        last_evaluated_at_ms: u64,
    ) -> Self {
        Self {
            baseline_sample,
            last_utilization,
            last_evaluated_at_ms,
        }
    }
}
