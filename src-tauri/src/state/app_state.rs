use crate::core::{ApplicationDiscoveryService, DiscoveryResult};
use crate::error::DomainError;
use crate::models::application::DiscoveredApplication;
use crate::security::InputGuard;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Mutex, RwLock};
use std::time::Instant;

/// Single synchronized in-memory catalog state.
///
/// Encapsulates generation tracking and application records within a unified
/// synchronization boundary, guaranteeing readers cannot observe torn state.
#[derive(Debug, Clone, Default)]
pub struct CatalogState {
    pub generation: u64,
    pub applications: HashMap<String, DiscoveredApplication>,
}

/// Application-wide runtime state managed by Tauri.
///
/// Contains internal operational health metadata, a discovery serialization mutex,
/// and the unified in-memory application catalog discovered from Windows.
#[derive(Debug)]
pub struct AppState {
    app_name: String,
    version: String,
    started_at: Instant,
    is_healthy: AtomicBool,
    is_quitting: AtomicBool,
    process_snapshot_seq: AtomicU64,
    catalog: RwLock<CatalogState>,
    discovery_lock: Mutex<()>,
}

impl AppState {
    pub fn new(app_name: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            app_name: app_name.into(),
            version: version.into(),
            started_at: Instant::now(),
            is_healthy: AtomicBool::new(true),
            is_quitting: AtomicBool::new(false),
            process_snapshot_seq: AtomicU64::new(1),
            catalog: RwLock::new(CatalogState::default()),
            discovery_lock: Mutex::new(()),
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

    pub fn is_quitting(&self) -> bool {
        self.is_quitting.load(Ordering::Relaxed)
    }

    pub fn set_quitting(&self, quitting: bool) {
        self.is_quitting.store(quitting, Ordering::Relaxed);
    }

    /// Allocates the next monotonic process snapshot sequence number using a checked atomic increment.
    ///
    /// # Explicit Overflow Policy
    /// Snapshot sequence numbers increment monotonically starting from 1 up to `u64::MAX - 1`.
    /// When the sequence reaches `u64::MAX`, rather than wrapping or reusing sequence numbers,
    /// the function refuses further allocation and returns `Err(DomainError::OperationFailed)`.
    /// Concurrent callers are guaranteed never to receive duplicate sequence values or wrapped values.
    /// Used strictly to scope ephemeral process identities across snapshot iterations.
    /// Does not store process handles or records in `AppState`.
    pub fn next_process_snapshot_seq(&self) -> Result<u64, DomainError> {
        let mut current = self.process_snapshot_seq.load(Ordering::Relaxed);
        loop {
            if current == u64::MAX {
                return Err(DomainError::OperationFailed(
                    "Process snapshot sequence numbers exhausted (u64 overflow)".to_string(),
                ));
            }
            let next = current + 1;
            match self.process_snapshot_seq.compare_exchange_weak(
                current,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(allocated) => return Ok(allocated),
                Err(actual) => current = actual,
            }
        }
    }

    /// Sets the process snapshot sequence number. Provided for deterministic boundary and overflow testing.
    pub fn set_process_snapshot_seq_for_testing(&self, seq: u64) {
        self.process_snapshot_seq.store(seq, Ordering::Relaxed);
    }

    /// Returns the current process snapshot sequence number without incrementing.
    pub fn current_process_snapshot_seq(&self) -> u64 {
        self.process_snapshot_seq.load(Ordering::Relaxed)
    }

    /// Atomically replaces the current application catalog with a new discovery generation.
    ///
    /// # Generational Semantics
    /// Generation count increments monotonically within the single write-lock acquisition.
    /// Generation overflow is handled deliberately by wrapping to 1 (reserving 0 for uninitialized).
    /// Internal domain models are indexed by their 128-bit application ID (`app_<32-hex>`).
    /// If duplicate IDs are encountered, later entries cleanly supersede earlier ones.
    pub fn update_catalog(
        &self,
        apps: Vec<DiscoveredApplication>,
    ) -> Result<(u64, usize), DomainError> {
        let mut map = HashMap::with_capacity(apps.len());
        for app in apps {
            map.insert(app.id.clone(), app);
        }
        let count = map.len();

        let mut lock = self
            .catalog
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        // Deliberate overflow handling: 0 is uninitialized, wraps to 1
        let next_gen = lock.generation.checked_add(1).unwrap_or(1);
        lock.generation = next_gen;
        lock.applications = map;

        Ok((next_gen, count))
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
        Ok(lock.applications.get(id).cloned())
    }

    /// Safely retrieves an internal application record paired with its catalog generation.
    pub fn get_application_with_generation(
        &self,
        id: &str,
    ) -> Result<Option<(u64, DiscoveredApplication)>, DomainError> {
        InputGuard::validate_application_id(id)?;

        let lock = self
            .catalog
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Ok(lock
            .applications
            .get(id)
            .cloned()
            .map(|app| (lock.generation, app)))
    }

    /// Returns the total number of applications currently stored in the in-memory catalog.
    pub fn catalog_len(&self) -> Result<usize, DomainError> {
        let lock = self
            .catalog
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Ok(lock.applications.len())
    }

    /// Returns the current catalog generation counter.
    pub fn catalog_generation(&self) -> u64 {
        let lock = self
            .catalog
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        lock.generation
    }

    /// Clears the in-memory catalog and increments the generation counter within the write lock.
    pub fn clear_catalog(&self) -> Result<u64, DomainError> {
        let mut lock = self
            .catalog
            .write()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let next_gen = lock.generation.checked_add(1).unwrap_or(1);
        lock.generation = next_gen;
        lock.applications.clear();
        Ok(next_gen)
    }

    /// Returns a full snapshot of the catalog and its generation under a single read lock.
    /// Applications are sorted deterministically by display name then ID.
    pub fn catalog_snapshot(&self) -> Result<(u64, Vec<DiscoveredApplication>), DomainError> {
        let lock = self
            .catalog
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut apps: Vec<DiscoveredApplication> = lock.applications.values().cloned().collect();
        apps.sort_by(|a, b| {
            a.display_name
                .to_lowercase()
                .cmp(&b.display_name.to_lowercase())
                .then_with(|| a.id.cmp(&b.id))
        });
        Ok((lock.generation, apps))
    }

    /// Executes serialized application discovery.
    ///
    /// Serializes discovery scans via `discovery_lock` so concurrent requests cannot execute
    /// scans out of order or overwrite newer results with older ones.
    pub fn execute_serialized_discovery(&self) -> Result<DiscoveryResult, DomainError> {
        self.execute_serialized_discovery_with(ApplicationDiscoveryService::discover_all)
    }

    /// Executes serialized discovery with a custom scanner function.
    ///
    /// # Concurrency & Deadlock Prevention
    /// `discovery_lock` is held for the duration of the scan to serialize callers.
    /// No catalog locks are held during the scan (`scan_fn`).
    /// Catalog write lock is acquired briefly in `update_catalog` only after `scan_fn` succeeds.
    /// A fatal scan error aborts without modifying the pre-existing catalog.
    pub fn execute_serialized_discovery_with<F>(
        &self,
        scan_fn: F,
    ) -> Result<DiscoveryResult, DomainError>
    where
        F: FnOnce() -> Result<DiscoveryResult, DomainError>,
    {
        // 1. Acquire discovery serialization mutex
        let _guard = match self.discovery_lock.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                // A prior discovery panicked. Deliberately recover guard to avoid permanent lockout.
                poisoned.into_inner()
            }
        };

        // 2. Perform discovery scan outside the catalog lock
        let result = scan_fn()?;

        // 3. Atomically replace catalog with internal domain models
        self.update_catalog(result.applications.clone())?;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::application::{AvailabilityState, DiscoverySource};
    use std::sync::Arc;

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
        assert!(!state.is_quitting());
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
    fn test_app_state_quitting_toggle() {
        let state = AppState::new("DeskPilot", "0.1.0");
        assert!(!state.is_quitting());
        state.set_quitting(true);
        assert!(state.is_quitting());
        state.set_quitting(false);
        assert!(!state.is_quitting());
    }

    #[test]
    fn test_process_snapshot_seq_lifecycle() {
        let state = AppState::new("DeskPilot", "0.1.0");
        assert_eq!(state.current_process_snapshot_seq(), 1);
        assert_eq!(state.next_process_snapshot_seq().unwrap(), 1);
        assert_eq!(state.next_process_snapshot_seq().unwrap(), 2);
        assert_eq!(state.next_process_snapshot_seq().unwrap(), 3);
        assert_eq!(state.current_process_snapshot_seq(), 4);
    }

    #[test]
    fn test_process_snapshot_seq_overflow_boundary() {
        let state = AppState::new("DeskPilot", "0.1.0");
        // Set counter to u64::MAX - 1
        state.set_process_snapshot_seq_for_testing(u64::MAX - 1);
        assert_eq!(state.current_process_snapshot_seq(), u64::MAX - 1);

        // Allocating u64::MAX - 1 succeeds and moves counter to u64::MAX
        let seq = state.next_process_snapshot_seq().unwrap();
        assert_eq!(seq, u64::MAX - 1);
        assert_eq!(state.current_process_snapshot_seq(), u64::MAX);

        // Once counter reaches u64::MAX, further allocations return Err
        let err1 = state.next_process_snapshot_seq();
        assert!(err1.is_err());
        assert_eq!(state.current_process_snapshot_seq(), u64::MAX);

        // Repeated attempts remain blocked and do not wrap
        let err2 = state.next_process_snapshot_seq();
        assert!(err2.is_err());
        assert_eq!(state.current_process_snapshot_seq(), u64::MAX);
    }

    #[test]
    fn test_process_snapshot_seq_concurrency() {
        use std::sync::Arc;
        use std::thread;

        let state = Arc::new(AppState::new("DeskPilot", "0.1.0"));
        let mut handles = Vec::new();

        for _ in 0..10 {
            let state_clone = Arc::clone(&state);
            handles.push(thread::spawn(move || {
                let mut seqs = Vec::new();
                for _ in 0..50 {
                    seqs.push(state_clone.next_process_snapshot_seq().unwrap());
                }
                seqs
            }));
        }

        let mut all_seqs = Vec::new();
        for h in handles {
            all_seqs.extend(h.join().unwrap());
        }

        assert_eq!(all_seqs.len(), 500);
        all_seqs.sort_unstable();
        all_seqs.dedup();
        // Strict uniqueness across concurrent threads
        assert_eq!(all_seqs.len(), 500);
        assert_eq!(all_seqs[0], 1);
        assert_eq!(all_seqs[499], 500);
    }

    #[test]
    fn test_catalog_update_and_lookup() {
        let state = AppState::new("DeskPilot", "0.1.0");
        let id = "app_0123456789abcdef0123456789abcdef";
        let app = make_test_app(id, "Test App", Some("C:\\App\\app.exe"));

        let (generation, count) = state.update_catalog(vec![app.clone()]).unwrap();
        assert_eq!(generation, 1);
        assert_eq!(count, 1);
        assert_eq!(state.catalog_len().unwrap(), 1);
        assert_eq!(state.catalog_generation(), 1);

        let retrieved = state.get_application(id).unwrap().expect("App must exist");
        assert_eq!(retrieved.id, id);
        assert_eq!(retrieved.display_name, "Test App");
        assert_eq!(retrieved.target_path.as_deref(), Some("C:\\App\\app.exe"));

        let with_gen = state
            .get_application_with_generation(id)
            .unwrap()
            .expect("Must exist with gen");
        assert_eq!(with_gen.0, 1);
        assert_eq!(with_gen.1.id, id);
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
        // Whitespace rejected
        assert!(
            state
                .get_application(" app_11111111111111111111111111111111")
                .is_err()
        );

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

        let (generation, count) = state.update_catalog(vec![app1, app2]).unwrap();
        assert_eq!(generation, 1);
        assert_eq!(count, 1, "Duplicate IDs must result in exactly 1 entry");
        let app = state.get_application(id).unwrap().unwrap();
        assert_eq!(app.display_name, "Version 2");
    }

    #[test]
    fn test_catalog_snapshot_sorting_and_atomic_generation() {
        let state = AppState::new("DeskPilot", "0.1.0");
        let id_z = "app_ffffffffffffffffffffffffffffffff";
        let id_a = "app_00000000000000000000000000000000";

        state
            .update_catalog(vec![
                make_test_app(id_z, "Zebra App", None),
                make_test_app(id_a, "Alpha App", None),
            ])
            .unwrap();

        let (generation, snapshot) = state.catalog_snapshot().unwrap();
        assert_eq!(generation, 1);
        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot[0].display_name, "Alpha App");
        assert_eq!(snapshot[1].display_name, "Zebra App");
    }

    #[test]
    fn test_concurrent_catalog_readers_and_writers() {
        let state = Arc::new(AppState::new("DeskPilot", "0.1.0"));
        let mut handles = vec![];

        // Spawn multiple reader threads
        for _ in 0..4 {
            let state_clone = Arc::clone(&state);
            handles.push(std::thread::spawn(move || {
                for _ in 0..50 {
                    let _ = state_clone.catalog_snapshot();
                    let _ = state_clone.catalog_len();
                    let _ = state_clone.catalog_generation();
                    std::thread::yield_now();
                }
            }));
        }

        // Spawn writer threads
        for i in 0..2 {
            let state_clone = Arc::clone(&state);
            handles.push(std::thread::spawn(move || {
                for j in 0..20 {
                    let id = format!("app_{:032x}", i * 100 + j);
                    let app = make_test_app(&id, "Concurrent App", None);
                    state_clone.update_catalog(vec![app]).unwrap();
                    std::thread::yield_now();
                }
            }));
        }

        for handle in handles {
            handle.join().expect("Concurrent thread must not panic");
        }

        assert!(state.catalog_generation() > 0);
    }

    #[test]
    fn test_serialized_discovery_execution_and_order() {
        let state = Arc::new(AppState::new("DeskPilot", "0.1.0"));

        let id_slow = "app_11111111111111111111111111111111";
        let id_fast = "app_22222222222222222222222222222222";

        let state_t1 = Arc::clone(&state);
        let t1 = std::thread::spawn(move || {
            state_t1
                .execute_serialized_discovery_with(|| {
                    // Simulate slow scan
                    std::thread::sleep(std::time::Duration::from_millis(60));
                    Ok(DiscoveryResult {
                        applications: vec![make_test_app(id_slow, "Slow App", None)],
                        scanned_sources: vec!["Source1".to_string()],
                        partial_failures: vec![],
                    })
                })
                .unwrap();
        });

        // Small delay to ensure T1 acquired discovery_lock first
        std::thread::sleep(std::time::Duration::from_millis(15));

        let state_t2 = Arc::clone(&state);
        let t2 = std::thread::spawn(move || {
            state_t2
                .execute_serialized_discovery_with(|| {
                    // Fast scan
                    Ok(DiscoveryResult {
                        applications: vec![make_test_app(id_fast, "Fast App", None)],
                        scanned_sources: vec!["Source2".to_string()],
                        partial_failures: vec![],
                    })
                })
                .unwrap();
        });

        t1.join().unwrap();
        t2.join().unwrap();

        // Because T1 ran first and held the discovery_lock, T2 waited and ran SECOND.
        // Therefore, T2's result is the final state and did NOT get overwritten by T1.
        assert_eq!(state.catalog_generation(), 2);
        assert!(state.get_application(id_fast).unwrap().is_some());
        assert!(state.get_application(id_slow).unwrap().is_none());
    }

    #[test]
    fn test_serialized_discovery_failure_preserves_catalog() {
        let state = AppState::new("DeskPilot", "0.1.0");
        let id = "app_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

        // Initial discovery succeeds
        state
            .execute_serialized_discovery_with(|| {
                Ok(DiscoveryResult {
                    applications: vec![make_test_app(id, "Initial App", None)],
                    scanned_sources: vec![],
                    partial_failures: vec![],
                })
            })
            .unwrap();
        assert_eq!(state.catalog_len().unwrap(), 1);
        assert_eq!(state.catalog_generation(), 1);

        // Subsequent discovery fails fatally
        let err = state.execute_serialized_discovery_with(|| {
            Err(DomainError::InternalError(
                "Simulated fatal scanner failure".to_string(),
            ))
        });
        assert!(err.is_err());

        // Previous catalog state MUST be preserved intact
        assert_eq!(state.catalog_len().unwrap(), 1);
        assert_eq!(state.catalog_generation(), 1);
        assert!(state.get_application(id).unwrap().is_some());
    }

    #[test]
    fn test_lock_poison_recovery() {
        let state = Arc::new(AppState::new("DeskPilot", "0.1.0"));
        let state_panic = Arc::clone(&state);

        // Intentionally poison the catalog lock by panicking while holding write guard
        let _ = std::thread::spawn(move || {
            let _lock = state_panic.catalog.write().unwrap();
            panic!("Intentional test panic to poison catalog RwLock");
        })
        .join();

        // Lock is now poisoned. Operations must recover gracefully via into_inner()
        let id = "app_1234567890abcdef1234567890abcdef";
        let (generation, count) = state
            .update_catalog(vec![make_test_app(id, "Recovered App", None)])
            .expect("Must recover from poisoned lock");
        assert_eq!(generation, 1);
        assert_eq!(count, 1);
        assert!(state.get_application(id).unwrap().is_some());
    }
}
