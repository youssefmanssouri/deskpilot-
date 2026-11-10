use deskpilot_lib::core::{ApplicationLauncherService, DiscoveryResult, StatusService};
use deskpilot_lib::error::DomainError;
use deskpilot_lib::models::{AppStatusDto, ApplicationDto};
use deskpilot_lib::models::application::{
    AvailabilityState, DiscoveredApplication, DiscoverySource,
};
use deskpilot_lib::state::AppState;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;

fn get_test_system_executable() -> PathBuf {
    let candidates = [
        r"C:\Windows\System32\whoami.exe",
        r"C:\Windows\System32\where.exe",
    ];
    for candidate in candidates {
        let p = PathBuf::from(candidate);
        if p.is_file() {
            return p;
        }
    }
    panic!("Standard Windows test executable not found in System32");
}

fn create_temp_exe_fixture(suffix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let name = format!(
        "deskpilot_integ_{}_{}_{}.exe",
        std::process::id(),
        suffix,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    path.push(name);
    let mut file = File::create(&path).expect("Failed to create temporary test binary");
    file.write_all(b"MZ\x00\x00deskpilot_synthetic_pe_test_binary")
        .expect("Failed to write fixture binary content");
    path
}

fn create_temp_script_fixture(suffix: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let name = format!(
        "deskpilot_integ_{}_{}_{}.bat",
        std::process::id(),
        suffix,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    path.push(name);
    let mut file = File::create(&path).expect("Failed to create temporary script file");
    file.write_all(b"@echo off\r\necho synthetic script")
        .expect("Failed to write fixture script content");
    path
}

fn make_discovered_app(
    id: &str,
    name: &str,
    target: Option<String>,
    availability: AvailabilityState,
) -> DiscoveredApplication {
    DiscoveredApplication {
        id: id.to_string(),
        display_name: name.to_string(),
        publisher: Some("DeskPilot Integration".to_string()),
        version: Some("2.0.0".to_string()),
        sources: vec![DiscoverySource::TestFixture],
        install_location: None,
        target_path: target,
        availability,
        discovered_at: 12345,
    }
}

#[test]
fn test_catalog_population_and_dto_consistency() {
    let state = AppState::new("DeskPilot", "0.1.0");
    let id_1 = "app_11111111111111111111111111111111";
    let id_2 = "app_22222222222222222222222222222222";

    let apps = vec![
        make_discovered_app(
            id_1,
            "App One",
            Some("C:\\AppOne\\app1.exe".to_string()),
            AvailabilityState::Available,
        ),
        make_discovered_app(
            id_2,
            "App Two",
            Some("C:\\AppTwo\\app2.exe".to_string()),
            AvailabilityState::MissingTarget,
        ),
    ];

    // Simulate discovery result updating catalog
    let (generation, count) = state
        .update_catalog(apps.clone())
        .expect("Catalog update must succeed");
    assert_eq!(generation, 1);
    assert_eq!(count, 2);
    assert_eq!(state.catalog_len().unwrap(), 2);
    assert_eq!(state.catalog_generation(), 1);

    // Map to DTOs as done during IPC dispatch
    let dtos: Vec<ApplicationDto> = apps.into_iter().map(ApplicationDto::from).collect();
    assert_eq!(dtos.len(), 2);

    // Verify DTO IDs and catalog IDs are 100% consistent
    for dto in &dtos {
        let catalog_app = state
            .get_application(&dto.id)
            .unwrap()
            .expect("App must exist in catalog");
        assert_eq!(catalog_app.id, dto.id);
        assert_eq!(catalog_app.display_name, dto.displayName);
        // Sensitive path must exist in internal catalog record
        assert!(catalog_app.target_path.is_some());
    }
}

#[test]
fn test_catalog_replacement_on_rediscovery() {
    let state = AppState::new("DeskPilot", "0.1.0");
    let id_old = "app_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let id_new = "app_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    // Initial discovery generation
    state
        .update_catalog(vec![make_discovered_app(
            id_old,
            "Old App",
            Some("C:\\Old\\app.exe".to_string()),
            AvailabilityState::Available,
        )])
        .unwrap();
    assert_eq!(state.catalog_generation(), 1);
    assert!(state.get_application(id_old).unwrap().is_some());

    // Second discovery generation (re-discovery no longer finds Old App)
    state
        .update_catalog(vec![make_discovered_app(
            id_new,
            "New App",
            Some("C:\\New\\app.exe".to_string()),
            AvailabilityState::Available,
        )])
        .unwrap();
    assert_eq!(state.catalog_generation(), 2);
    // Old app must NOT be silently retained
    assert!(state.get_application(id_old).unwrap().is_none());
    assert!(state.get_application(id_new).unwrap().is_some());
    assert_eq!(state.catalog_len().unwrap(), 1);
}

#[test]
fn test_launcher_resolves_and_validates_from_catalog() {
    let state = AppState::new("DeskPilot", "0.1.0");
    let sys_exe = get_test_system_executable();
    let id = "app_33333333333333333333333333333333";

    state
        .update_catalog(vec![make_discovered_app(
            id,
            "Launchable App",
            Some(sys_exe.to_string_lossy().to_string()),
            AvailabilityState::Available,
        )])
        .unwrap();

    let validated = ApplicationLauncherService::resolve_and_validate(&state, id)
        .expect("Valid target must pass pre-flight resolution and validation");

    assert_eq!(validated.application_id, id);
    assert_eq!(validated.display_name, "Launchable App");
    assert!(validated.executable_path.is_file());
    assert!(validated.working_directory.is_some());
}

#[test]
fn test_launcher_rejects_missing_file_from_catalog() {
    let state = AppState::new("DeskPilot", "0.1.0");
    let id = "app_44444444444444444444444444444444";

    // Target path exists in metadata but file is not on disk
    state
        .update_catalog(vec![make_discovered_app(
            id,
            "Missing App",
            Some("C:\\NonExistentPath_DeskPilot_Test\\app.exe".to_string()),
            AvailabilityState::Available,
        )])
        .unwrap();

    let err = ApplicationLauncherService::resolve_and_validate(&state, id).unwrap_err();
    match err {
        DomainError::Unavailable(msg) => {
            assert!(
                msg.contains("does not exist on disk") || msg.contains("could not be resolved")
            );
            // Error must NEVER leak the path
            assert!(!msg.contains("C:\\"));
            assert!(!msg.contains("NonExistentPath"));
        }
        _ => panic!("Expected Unavailable error for missing target file"),
    }
}

#[test]
fn test_launcher_rejects_non_exe_script_from_catalog() {
    let state = AppState::new("DeskPilot", "0.1.0");
    let temp_bat = create_temp_script_fixture("script");
    let id = "app_55555555555555555555555555555555";

    state
        .update_catalog(vec![make_discovered_app(
            id,
            "Script App",
            Some(temp_bat.to_string_lossy().to_string()),
            AvailabilityState::Available,
        )])
        .unwrap();

    let err = ApplicationLauncherService::resolve_and_validate(&state, id).unwrap_err();
    match err {
        DomainError::PermissionDenied(msg) => {
            assert!(msg.contains("not a supported executable format"));
            assert!(!msg.contains(".bat"));
        }
        _ => panic!("Expected PermissionDenied error for non-exe target"),
    }

    let _ = std::fs::remove_file(temp_bat);
}

#[test]
fn test_empty_discovery_clears_catalog() {
    let state = AppState::new("DeskPilot", "0.1.0");
    let id = "app_66666666666666666666666666666666";

    state
        .update_catalog(vec![make_discovered_app(
            id,
            "Initial App",
            Some("C:\\Init\\app.exe".to_string()),
            AvailabilityState::Available,
        )])
        .unwrap();
    assert_eq!(state.catalog_len().unwrap(), 1);

    // Empty discovery results in empty catalog
    state.update_catalog(vec![]).unwrap();
    assert_eq!(state.catalog_len().unwrap(), 0);
    assert_eq!(state.catalog_generation(), 2);
    assert!(state.get_application(id).unwrap().is_none());
}

#[test]
fn test_launcher_rejects_network_unc_targets_upfront() {
    let state = AppState::new("DeskPilot", "0.1.0");
    let test_cases = vec![
        r"\\192.168.1.50\share\app.exe",
        "//192.168.1.50/share/app.exe",
        r"\\?\UNC\corp-server\tools\app.exe",
        "//?/UNC/corp-server/tools/app.exe",
        r"\\.\COM1",
        r"\\.\PhysicalDrive0",
        r"\\?\Volume{00000000-0000-0000-0000-000000000000}\app.exe",
        r"\\?\GlobalRoot\Device\Harddisk0\app.exe",
    ];

    for (idx, target) in test_cases.into_iter().enumerate() {
        let id = format!("app_{:032x}", idx + 100);
        state
            .update_catalog(vec![make_discovered_app(
                &id,
                "Network App",
                Some(target.to_string()),
                AvailabilityState::Available,
            )])
            .unwrap();

        let start = std::time::Instant::now();
        let err = ApplicationLauncherService::resolve_and_validate(&state, &id).unwrap_err();
        let elapsed = start.elapsed();

        // Must reject instantaneously via lexical check (< 50ms) without contacting network
        assert!(
            elapsed < std::time::Duration::from_millis(50),
            "Target '{}' took too long: {:?}",
            target,
            elapsed
        );
        match err {
            DomainError::PermissionDenied(msg) => {
                assert!(
                    msg.contains("prohibited") || msg.contains("network") || msg.contains("device"),
                    "Target '{}' yielded unexpected message: {}",
                    target,
                    msg
                );
            }
            _ => panic!(
                "Expected PermissionDenied for target '{}', got {:?}",
                target, err
            ),
        }
    }
}

#[test]
fn test_catalog_snapshot_atomic_generation_guarantee() {
    let state = AppState::new("DeskPilot", "0.1.0");
    let id = "app_abcdefabcdefabcdefabcdefabcdefab";

    state
        .update_catalog(vec![make_discovered_app(
            id,
            "Snapshot App",
            Some("C:\\App\\app.exe".to_string()),
            AvailabilityState::Available,
        )])
        .unwrap();

    let (generation, snapshot) = state.catalog_snapshot().unwrap();
    assert_eq!(generation, 1);
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot[0].id, id);
    assert_eq!(snapshot[0].display_name, "Snapshot App");
}

#[test]
fn test_concurrent_discovery_serialization() {
    let state = Arc::new(AppState::new("DeskPilot", "0.1.0"));

    let id_a = "app_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    let id_b = "app_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";

    let state_1 = Arc::clone(&state);
    let h1 = std::thread::spawn(move || {
        state_1
            .execute_serialized_discovery_with(|| {
                // Thread 1: slow scan
                std::thread::sleep(std::time::Duration::from_millis(50));
                Ok(DiscoveryResult {
                    applications: vec![make_discovered_app(
                        id_a,
                        "App A",
                        None,
                        AvailabilityState::Available,
                    )],
                    scanned_sources: vec!["SourceA".to_string()],
                    partial_failures: vec![],
                })
            })
            .unwrap();
    });

    // Ensure thread 1 acquires the discovery lock first
    std::thread::sleep(std::time::Duration::from_millis(10));

    let state_2 = Arc::clone(&state);
    let h2 = std::thread::spawn(move || {
        state_2
            .execute_serialized_discovery_with(|| {
                // Thread 2: fast scan
                Ok(DiscoveryResult {
                    applications: vec![make_discovered_app(
                        id_b,
                        "App B",
                        None,
                        AvailabilityState::Available,
                    )],
                    scanned_sources: vec!["SourceB".to_string()],
                    partial_failures: vec![],
                })
            })
            .unwrap();
    });

    h1.join().unwrap();
    h2.join().unwrap();

    // Since discovery is serialized, Thread 2 executed AFTER Thread 1.
    // The final catalog generation must be 2, containing only App B.
    assert_eq!(state.catalog_generation(), 2);
    assert!(state.get_application(id_b).unwrap().is_some());
    assert!(state.get_application(id_a).unwrap().is_none());
}

#[test]
fn test_launch_integration_end_to_end() {
    let state = AppState::new("DeskPilot", "0.1.0");
    let sys_exe = get_test_system_executable();
    let id = "app_11112222333344445555666677778888";

    state
        .update_catalog(vec![make_discovered_app(
            id,
            "Integration Launch Target",
            Some(sys_exe.to_string_lossy().to_string()),
            AvailabilityState::Available,
        )])
        .unwrap();

    let result = ApplicationLauncherService::launch(&state, id)
        .expect("Integration launch must succeed for registered local executable");

    assert_eq!(result.application_id, id);
    assert!(result.process_id > 0);
}

#[test]
fn test_launch_integration_rejects_transient_temp_directory() {
    let state = AppState::new("DeskPilot", "0.1.0");
    let temp_exe = create_temp_exe_fixture("integ_transient");
    let id = "app_99998888777766665555444433332222";

    state
        .update_catalog(vec![make_discovered_app(
            id,
            "Transient App",
            Some(temp_exe.to_string_lossy().to_string()),
            AvailabilityState::Available,
        )])
        .unwrap();

    let err = ApplicationLauncherService::launch(&state, id).unwrap_err();
    match err {
        DomainError::PermissionDenied(msg) => {
            assert!(msg.contains("temporary or transient directories is prohibited"));
            assert!(!msg.contains("AppData"));
            assert!(!msg.contains("Temp"));
        }
        _ => panic!("Expected PermissionDenied for transient target in integration test"),
    }

    let _ = std::fs::remove_file(temp_exe);
}

#[test]
fn test_launch_concurrency_with_rediscovery() {
    let state = Arc::new(AppState::new("DeskPilot", "0.1.0"));
    let sys_exe = get_test_system_executable();
    let id_target = "app_abababababababababababababababab";
    let id_new = "app_cdcdcdcdcdcdcdcdcdcdcdcdcdcdcdcd";

    // Initial catalog generation containing the target
    state
        .update_catalog(vec![make_discovered_app(
            id_target,
            "Concurrent Launch App",
            Some(sys_exe.to_string_lossy().to_string()),
            AvailabilityState::Available,
        )])
        .unwrap();

    let state_launch = Arc::clone(&state);
    let launch_handle =
        std::thread::spawn(move || ApplicationLauncherService::launch(&state_launch, id_target));

    // Brief yield/pause to allow launch thread to resolve target before rediscovery replaces catalog
    std::thread::sleep(std::time::Duration::from_millis(5));

    let state_discovery = Arc::clone(&state);
    let discovery_handle = std::thread::spawn(move || {
        // Rediscovery updates catalog to generation 2 with a new app
        state_discovery.update_catalog(vec![make_discovered_app(
            id_new,
            "New Rediscovered App",
            None,
            AvailabilityState::Available,
        )])
    });

    let launch_res = launch_handle
        .join()
        .unwrap()
        .expect("Concurrent launch must succeed");
    let (gen2, count2) = discovery_handle
        .join()
        .unwrap()
        .expect("Rediscovery must succeed");

    assert_eq!(launch_res.application_id, id_target);
    assert!(launch_res.process_id > 0);
    assert_eq!(gen2, 2);
    assert_eq!(count2, 1);
}

#[test]
fn test_live_host_discovery_and_catalog_stability() {
    use deskpilot_lib::core::ApplicationDiscoveryService;

    let disc_res = ApplicationDiscoveryService::discover_all();
    assert!(
        disc_res.is_ok(),
        "Live host discovery must execute successfully without panic"
    );
    let disc = disc_res.unwrap();

    println!(
        "HOST_DISCOVERY: total_discovered={}",
        disc.applications.len()
    );
    println!("HOST_DISCOVERY: scanned_sources={:?}", disc.scanned_sources);
    println!(
        "HOST_DISCOVERY: partial_failures={:?}",
        disc.partial_failures
    );

    for app in disc.applications.iter().take(10) {
        println!(
            "HOST_DISCOVERY_APP: id={} name='{}' pub='{:?}' ver='{:?}' avail={:?} sources={:?}",
            app.id, app.display_name, app.publisher, app.version, app.availability, app.sources
        );
    }

    // Verify catalog integration through AppState with repeated discovery cycles
    let state = AppState::new("DeskPilotLiveTest", "1.0.0");
    let initial_res = state
        .execute_serialized_discovery()
        .expect("AppState serialized live discovery should succeed");
    assert_eq!(
        initial_res.applications.len(),
        disc.applications.len(),
        "Catalog snapshot must match direct discovery count"
    );

    let second_res = state
        .execute_serialized_discovery()
        .expect("Second AppState serialized live discovery should succeed");
    assert_eq!(
        second_res.applications.len(),
        initial_res.applications.len(),
        "Repeated live discovery must produce stable application count"
    );
    assert_eq!(
        state.catalog_generation(),
        2,
        "Catalog generation must increment to 2 after second live discovery"
    );
}

#[test]
fn test_app_state_and_status_service_version_synchronization() {
    // 1. AppState receives the Cargo package version
    let state = AppState::new("DeskPilot", env!("CARGO_PKG_VERSION"));
    assert_eq!(state.app_name(), "DeskPilot");
    assert_eq!(state.version(), env!("CARGO_PKG_VERSION"));
    assert_eq!(state.version(), "0.1.1");

    // 2. StatusService query returns 0.1.1
    let status = StatusService::get_status(&state)
        .expect("StatusService::get_status must succeed");
    assert_eq!(status.version, "0.1.1");
    assert_eq!(status.version, env!("CARGO_PKG_VERSION"));

    // 3. Application status DTO reports 0.1.1
    let dto: AppStatusDto = status.into();
    assert_eq!(dto.version, "0.1.1");
    assert_eq!(dto.version, env!("CARGO_PKG_VERSION"));
    assert_eq!(dto.app_name, "DeskPilot");
    assert_eq!(dto.status, "ready");
}
