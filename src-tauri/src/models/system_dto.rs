use crate::models::system::{
    CpuMetrics, DriveMetrics, MemoryMetrics, OsInfoMetrics, StorageMetrics, SystemMetrics,
};
use serde::{Deserialize, Serialize};

/// Serialized Data Transfer Object for CPU metrics sent across Tauri IPC.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CpuMetricsDto {
    /// Processor marketing name or brand string.
    pub brand_name: String,
    /// Number of logical processors.
    pub logical_cores: u32,
    /// Architecture identifier (e.g. "x86_64", "ARM64").
    pub architecture: String,
    /// Calculated CPU utilization percentage (0.0 to 100.0).
    pub utilization_percent: f64,
}

impl From<CpuMetrics> for CpuMetricsDto {
    fn from(m: CpuMetrics) -> Self {
        Self {
            brand_name: m.brand_name,
            logical_cores: m.logical_cores,
            architecture: m.architecture,
            utilization_percent: m.utilization_percent,
        }
    }
}

/// Serialized Data Transfer Object for physical RAM metrics.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MemoryMetricsDto {
    /// Total installed physical memory in bytes.
    pub total_bytes: u64,
    /// Available/free physical memory in bytes.
    pub available_bytes: u64,
    /// Resident memory in active use in bytes.
    pub used_bytes: u64,
    /// Memory load percentage (0.0 to 100.0).
    pub used_percent: f64,
}

impl From<MemoryMetrics> for MemoryMetricsDto {
    fn from(m: MemoryMetrics) -> Self {
        Self {
            total_bytes: m.total_bytes,
            available_bytes: m.available_bytes,
            used_bytes: m.used_bytes,
            used_percent: m.used_percent,
        }
    }
}

/// Serialized Data Transfer Object for a single local storage drive.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DriveMetricsDto {
    /// Volume root path (e.g. "C:\").
    pub root_path: String,
    /// User-assigned or filesystem volume label.
    pub volume_name: String,
    /// Filesystem format name (e.g. "NTFS").
    pub file_system: String,
    /// Total volume capacity in bytes.
    pub total_bytes: u64,
    /// Available capacity for unprivileged callers in bytes.
    pub available_bytes: u64,
    /// Used capacity in bytes.
    pub used_bytes: u64,
    /// Capacity usage percentage (0.0 to 100.0).
    pub used_percent: f64,
}

impl From<DriveMetrics> for DriveMetricsDto {
    fn from(m: DriveMetrics) -> Self {
        Self {
            root_path: m.root_path,
            volume_name: m.volume_name,
            file_system: m.file_system,
            total_bytes: m.total_bytes,
            available_bytes: m.available_bytes,
            used_bytes: m.used_bytes,
            used_percent: m.used_percent,
        }
    }
}

/// Serialized Data Transfer Object for storage subsystem.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StorageMetricsDto {
    pub drives: Vec<DriveMetricsDto>,
}

impl From<StorageMetrics> for StorageMetricsDto {
    fn from(m: StorageMetrics) -> Self {
        Self {
            drives: m.drives.into_iter().map(DriveMetricsDto::from).collect(),
        }
    }
}

/// Serialized Data Transfer Object for system uptime and platform metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OsInfoMetricsDto {
    /// Operating system uptime in seconds since boot.
    pub uptime_seconds: u64,
    /// DeskPilot application uptime in seconds.
    pub app_uptime_seconds: u64,
    /// Operating system platform name.
    pub os_name: String,
    /// Host processor architecture.
    pub architecture: String,
}

impl From<OsInfoMetrics> for OsInfoMetricsDto {
    fn from(m: OsInfoMetrics) -> Self {
        Self {
            uptime_seconds: m.uptime_seconds,
            app_uptime_seconds: m.app_uptime_seconds,
            os_name: m.os_name,
            architecture: m.architecture,
        }
    }
}

/// Top-level System Intelligence snapshot sent across the IPC boundary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SystemMetricsDto {
    pub cpu: CpuMetricsDto,
    pub memory: MemoryMetricsDto,
    pub storage: StorageMetricsDto,
    pub system: OsInfoMetricsDto,
    /// Unix epoch timestamp in milliseconds when this snapshot was captured.
    pub sampled_at_ms: u64,
}

impl From<SystemMetrics> for SystemMetricsDto {
    fn from(m: SystemMetrics) -> Self {
        Self {
            cpu: CpuMetricsDto::from(m.cpu),
            memory: MemoryMetricsDto::from(m.memory),
            storage: StorageMetricsDto::from(m.storage),
            system: OsInfoMetricsDto::from(m.system),
            sampled_at_ms: m.sampled_at_ms,
        }
    }
}
