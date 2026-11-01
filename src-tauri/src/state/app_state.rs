use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

/// Application-wide runtime state managed by Tauri.
#[derive(Debug)]
pub struct AppState {
    app_name: String,
    version: String,
    started_at: Instant,
    is_healthy: AtomicBool,
}

impl AppState {
    pub fn new(app_name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            app_name: app_name.into(),
            version: version.into(),
            started_at: Instant::now(),
            is_healthy: AtomicBool::new(true),
        }
    }

    pub fn app_name(&self) -> &str {
        &self.app_name
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn uptime_seconds(&self) -> u64 {
        self.started_at.elapsed().as_secs()
    }

    pub fn is_healthy(&self) -> bool {
        self.is_healthy.load(Ordering::Relaxed)
    }

    pub fn set_healthy(&self, healthy: bool) {
        self.is_healthy.store(healthy, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_initialization() {
        let state = AppState::new("DeskPilot", "0.1.0");
        assert_eq!(state.app_name(), "DeskPilot");
        assert_eq!(state.version(), "0.1.0");
        assert!(state.is_healthy());
        // Uptime should be >= 0
        let _ = state.uptime_seconds();
    }

    #[test]
    fn test_app_state_health_toggle() {
        let state = AppState::new("DeskPilot", "0.1.0");
        state.set_healthy(false);
        assert!(!state.is_healthy());
        state.set_healthy(true);
        assert!(state.is_healthy());
    }
}
