use crate::error::DomainError;
use crate::models::system::{CpuMetrics, StorageMetrics, SystemMetrics, SystemTimesSample};
use crate::models::system_dto::SystemMetricsDto;
use crate::platform::windows::WindowsSystemProvider;
use crate::state::AppState;

/// Core service for gathering and computing System Intelligence metrics.
pub struct SystemService;

impl SystemService {
    /// Pure, deterministic calculation of CPU utilization percentage across two discrete time samples.
    ///
    /// # Windows NT Kernel Formula
    /// In Windows NT, `kernel_time` includes `idle_time`. Therefore:
    /// - $\Delta \text{Kernel} = \text{kernel}_1 - \text{kernel}_0$
    /// - $\Delta \text{User} = \text{user}_1 - \text{user}_0$
    /// - $\Delta \text{Idle} = \text{idle}_1 - \text{idle}_0$
    /// - $\Delta \text{Total} = \Delta \text{Kernel} + \Delta \text{User}$
    /// - $\Delta \text{Busy} = \Delta \text{Total} - \Delta \text{Idle}$
    /// - $\text{Utilization} = (\Delta \text{Busy} / \Delta \text{Total}) \times 100.0$
    ///
    /// Guarantees:
    /// - Returns 0.0 if total delta is zero (no time elapsed).
    /// - Clamps result strictly within `0.0` to `100.0`.
    /// - Rounds to 1 decimal place for stable presentation.
    pub fn calculate_cpu_utilization(prev: &SystemTimesSample, curr: &SystemTimesSample) -> f64 {
        let delta_kernel = curr.kernel_time.saturating_sub(prev.kernel_time);
        let delta_user = curr.user_time.saturating_sub(prev.user_time);
        let delta_idle = curr.idle_time.saturating_sub(prev.idle_time);
        let delta_total = delta_kernel.saturating_add(delta_user);

        if delta_total == 0 {
            return 0.0;
        }

        let delta_busy = delta_total.saturating_sub(delta_idle);
        let percent = (delta_busy as f64 / delta_total as f64) * 100.0;
        let clamped = percent.clamp(0.0, 100.0);
        ((clamped * 10.0).round()) / 10.0
    }

    /// Samples CPU utilization using the synchronized coordinator in `AppState`.
    pub fn sample_cpu_utilization(state: &AppState) -> Result<(f64, u64), DomainError> {
        state.sample_cpu_utilization()
    }

    /// Queries all system intelligence dimensions and maps them into a safe DTO.
    ///
    /// # Synchronized CPU Sampling
    /// CPU sampling is delegated to `AppState::sample_cpu_utilization`, which guarantees:
    /// - Mutual exclusion during cold-start baseline acquisition.
    /// - Minimum 500ms sampling interval enforcement.
    /// - Cache reuse within the minimum interval without taking redundant samples.
    pub fn get_metrics(state: &AppState) -> Result<SystemMetricsDto, DomainError> {
        let (utilization_percent, sampled_at_ms) = state.sample_cpu_utilization()?;

        // Query CPU hardware metadata
        let (brand_name, logical_cores, architecture) = WindowsSystemProvider::query_cpu_info()?;
        let cpu = CpuMetrics {
            brand_name,
            logical_cores,
            architecture,
            utilization_percent,
        };

        // Query memory metrics
        let memory = WindowsSystemProvider::query_memory()?;

        // Query local storage drives
        let drives = WindowsSystemProvider::query_drives()?;
        let storage = StorageMetrics { drives };

        // Query OS and runtime info
        let system = WindowsSystemProvider::query_os_info(state.uptime_seconds())?;

        let metrics = SystemMetrics {
            cpu,
            memory,
            storage,
            system,
            sampled_at_ms,
        };

        Ok(SystemMetricsDto::from(metrics))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_cpu_utilization_zero_delta() {
        let sample = SystemTimesSample::new(100, 200, 300, 1000);
        assert_eq!(
            SystemService::calculate_cpu_utilization(&sample, &sample),
            0.0
        );
    }

    #[test]
    fn test_calculate_cpu_utilization_full_idle() {
        // Delta Kernel = 100, Delta Idle = 100, Delta User = 0 => Total = 100, Busy = 0
        let s0 = SystemTimesSample::new(1000, 1000, 0, 1000);
        let s1 = SystemTimesSample::new(1100, 1100, 0, 2000);
        assert_eq!(SystemService::calculate_cpu_utilization(&s0, &s1), 0.0);
    }

    #[test]
    fn test_calculate_cpu_utilization_full_busy() {
        // Delta Kernel = 50, Delta Idle = 0, Delta User = 50 => Total = 100, Busy = 100
        let s0 = SystemTimesSample::new(1000, 1000, 500, 1000);
        let s1 = SystemTimesSample::new(1000, 1050, 550, 2000);
        assert_eq!(SystemService::calculate_cpu_utilization(&s0, &s1), 100.0);
    }

    #[test]
    fn test_calculate_cpu_utilization_half_load() {
        // Delta Kernel = 100 (including 50 idle), Delta User = 0 => Total = 100, Busy = 50 => 50.0%
        let s0 = SystemTimesSample::new(1000, 1000, 0, 1000);
        let s1 = SystemTimesSample::new(1050, 1100, 0, 2000);
        assert_eq!(SystemService::calculate_cpu_utilization(&s0, &s1), 50.0);
    }

    #[test]
    fn test_calculate_cpu_utilization_backward_clock_resilience() {
        let s0 = SystemTimesSample::new(2000, 2000, 1000, 2000);
        let s1 = SystemTimesSample::new(1000, 1000, 500, 1000);
        assert_eq!(SystemService::calculate_cpu_utilization(&s0, &s1), 0.0);
    }

    #[test]
    fn test_get_metrics_cold_start_and_subsequent_calls() {
        let state = AppState::new("DeskPilot", "0.1.0");

        // First call: cold start (takes 100ms baseline wait via coordinator)
        let metrics1 = SystemService::get_metrics(&state)
            .expect("First get_metrics call must succeed on Windows");
        assert!(metrics1.cpu.logical_cores > 0);
        assert!(!metrics1.cpu.brand_name.is_empty());
        assert!(
            metrics1.cpu.utilization_percent >= 0.0 && metrics1.cpu.utilization_percent <= 100.0
        );
        assert!(metrics1.memory.total_bytes > 0);
        assert!(!metrics1.storage.drives.is_empty());
        assert_eq!(metrics1.system.os_name, "Windows");
        assert!(metrics1.system.uptime_seconds > 0);
        assert!(state.cpu_coordinator_state().is_some());

        // Second immediate call (< 500ms): reuses cached utilization and preserves baseline
        let metrics2 = SystemService::get_metrics(&state)
            .expect("Second get_metrics call must succeed on Windows");
        assert_eq!(
            metrics2.cpu.utilization_percent,
            metrics1.cpu.utilization_percent
        );
        assert_eq!(metrics2.sampled_at_ms, metrics1.sampled_at_ms);
    }
}
