use crate::error::DomainError;
use crate::models::application::DiscoveredApplication;
use crate::security::InputGuard;
use std::collections::HashMap;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

/// Application-wide runtime state managed by Tauri.
///
/// Contains internal operational health metadata and the privileged in-memory
/// application catalog discovered from Windows.
#[derive(Debug)]
pub struct AppState {
    app_name: String,
    version: String,
    started_at: Instant,
    is_healthy: AtomicBool,
    catalog_generation: AtomicU64,
    catalog: RwLock<HashMap<String, DiscoveredApplication>>,
}

impl AppState {
    pub fn new(app_name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            app_name: app_name.into(),
            version: version.into(),
            started_at: Instant::now(),
            is_healthy: AtomicBool::new(true),
            catalog_generation: AtomicU64::new(0),
            catalog: RwLock::new(HashMap::new()),
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

    /// Atomically replaces the current application catalog with a new discovery generation.
    ///
    /// # Generational Semantics
    /// Each successful replacement increments the catalog generation count.
    /// Internal domain models are indexed by their 128-bit application ID (`app_<32-hex>`).
    /// If duplicate IDs are encountered, later entries cleanly supersede earlier ones.
    pub fn update_catalog(&self, apps: Vec<DiscoveredApplication>) -> Result<usize, DomainError> {
        let mut map = HashMap::with_capacity(apps.len());
        for app in apps {
            map.insert(app.id.clone(), app);
        }
        let count = map.len();

        let mut lock = self
            .catalog
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *lock = map;
        self.catalog_generation.fetch_add(1, Ordering::SeqCst);

        Ok(count)
    }

    /// Safely retrieves an internal application record by its validated identifier.
    ///
    /// # Security Boundary
    /// 1. Strictly validates the identifier format before attempting any lookup.
    /// 2. Returns a cloned `DiscoveredApplication` record.
    /// 3. The catalog read lock is released immediately upon return, ensuring callers
    ///    never hold a catalog lock while performing disk I/O or pre-flight checks.
    pub fn get_application(&self, id: &str) -> Result<Option<DiscoveredApplication>, DomainError> {
        InputGuard::validate_application_id(id)?;

        let lock = self
            .catalog
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Ok(lock.get(id).cloned())
    }

    /// Returns the total number of applications currently stored in the in-memory catalog.
    pub fn catalog_len(&self) -> Result<usize, DomainError> {
        let lock = self
            .catalog
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Ok(lock.len())
    }

    /// Returns the current catalog generation counter.
    pub fn catalog_generation(&self) -> u64 {
        self.catalog_generation.load(Ordering::SeqCst)
    }

    /// Clears the in-memory catalog and increments the generation counter.
    pub fn clear_catalog(&self) -> Result<(), DomainError> {
        let mut lock = self
            .catalog
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        lock.clear();
        self.catalog_generation.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    /// Returns a full snapshot of the catalog, sorted deterministically by display name then ID.
    pub fn catalog_snapshot(&self) -> Result<Vec<DiscoveredApplication>, DomainError> {
        let lock = self
            .catalog
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut apps: Vec<DiscoveredApplication> = lock.values().cloned().collect();
        apps.sort_by(|a, b| {
            a.display_name
                .to_lowercase()
                .cmp(&b.display_name.to_lowercase())
                .then_with(|| a.id.cmp(&b.id))
        });
        Ok(apps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::application::{AvailabilityState, DiscoverySource};

    fn make_test_app(id: &str, name: &str, target: Option<&str>) -> DiscoveredApplication {
        DiscoveredApplication {
            id: id.to_string(),
            display_name: name.to_string(),
            publisher: Some("Test Publisher".to_string()),
            version: Some("1.0.0".to_string()),
            sources: vec![DiscoverySource::TestFixture],
            install_location: None,
            target_path: target.map(|t| t.to_string()),
            availability: AvailabilityState::Available,
            discovered_at: 1000,
        }
    }

    #[test]
    fn test_app_state_initialization() {
        let state = AppState::new("DeskPilot", "0.1.0");
        assert_eq!(state.app_name(), "DeskPilot");
        assert_eq!(state.version(), "0.1.0");
        assert!(state.is_healthy());
        let _ = state.uptime_seconds();
        assert_eq!(state.catalog_len().unwrap(), 0);
        assert_eq!(state.catalog_generation(), 0);
    }

    #[test]
    fn test_app_state_health_toggle() {
        let state = AppState::new("DeskPilot", "0.1.0");
        state.set_healthy(false);
        assert!(!state.is_healthy());
        state.set_healthy(true);
        assert!(state.is_healthy());
    }

    #[test]
    fn test_catalog_update_and_lookup() {
        let state = AppState::new("DeskPilot", "0.1.0");
        let id = "app_0123456789abcdef0123456789abcdef";
        let app = make_test_app(id, "Test App", Some("C:\\App\\app.exe"));

        let count = state.update_catalog(vec![app.clone()]).unwrap();
        assert_eq!(count, 1);
        assert_eq!(state.catalog_len().unwrap(), 1);
        assert_eq!(state.catalog_generation(), 1);

        let retrieved = state.get_application(id).unwrap().expect("App must exist");
        assert_eq!(retrieved.id, id);
        assert_eq!(retrieved.display_name, "Test App");
        assert_eq!(retrieved.target_path.as_deref(), Some("C:\\App\\app.exe"));
    }

    #[test]
    fn test_catalog_malformed_and_unknown_id_rejection() {
        let state = AppState::new("DeskPilot", "0.1.0");
        let valid_unknown = "app_11111111111111111111111111111111";

        // Malformed IDs return InvalidInput error
        assert!(state.get_application("invalid-id").is_err());
        assert!(
            state
                .get_application("../app_0123456789abcdef0123456789abcdef")
                .is_err()
        );
        assert!(state.get_application("").is_err());

        // Valid syntax ID that is not in catalog returns Ok(None)
        let result = state.get_application(valid_unknown).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_catalog_atomic_replacement() {
        let state = AppState::new("DeskPilot", "0.1.0");
        let id_a = "app_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let id_b = "app_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

        state
            .update_catalog(vec![make_test_app(id_a, "App A", None)])
            .unwrap();
        assert_eq!(state.catalog_generation(), 1);
        assert!(state.get_application(id_a).unwrap().is_some());

        // Replace with new generation containing only App B
        state
            .update_catalog(vec![make_test_app(id_b, "App B", None)])
            .unwrap();
        assert_eq!(state.catalog_generation(), 2);
        assert!(state.get_application(id_a).unwrap().is_none());
        assert!(state.get_application(id_b).unwrap().is_some());
    }

    #[test]
    fn test_catalog_empty_and_clear_behavior() {
        let state = AppState::new("DeskPilot", "0.1.0");
        let id = "app_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

        state
            .update_catalog(vec![make_test_app(id, "App A", None)])
            .unwrap();
        assert_eq!(state.catalog_len().unwrap(), 1);

        state.clear_catalog().unwrap();
        assert_eq!(state.catalog_len().unwrap(), 0);
        assert_eq!(state.catalog_generation(), 2);
        assert!(state.get_application(id).unwrap().is_none());
    }

    #[test]
    fn test_catalog_duplicate_id_handling() {
        let state = AppState::new("DeskPilot", "0.1.0");
        let id = "app_cccccccccccccccccccccccccccccccc";
        let app1 = make_test_app(id, "Version 1", Some("C:\\v1.exe"));
        let app2 = make_test_app(id, "Version 2", Some("C:\\v2.exe"));

        let count = state.update_catalog(vec![app1, app2]).unwrap();
        assert_eq!(count, 1, "Duplicate IDs must result in exactly 1 entry");
        let app = state.get_application(id).unwrap().unwrap();
        assert_eq!(app.display_name, "Version 2");
    }

    #[test]
    fn test_catalog_snapshot_sorting() {
        let state = AppState::new("DeskPilot", "0.1.0");
        let id_z = "app_ffffffffffffffffffffffffffffffff";
        let id_a = "app_00000000000000000000000000000000";

        state
            .update_catalog(vec![
                make_test_app(id_z, "Zebra App", None),
                make_test_app(id_a, "Alpha App", None),
            ])
            .unwrap();

        let snapshot = state.catalog_snapshot().unwrap();
        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot[0].display_name, "Alpha App");
        assert_eq!(snapshot[1].display_name, "Zebra App");
    }
}
