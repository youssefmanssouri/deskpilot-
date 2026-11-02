use deskpilot_lib::core::ApplicationLauncherService;
use deskpilot_lib::error::DomainError;
use deskpilot_lib::models::ApplicationDto;
use deskpilot_lib::models::application::{
    AvailabilityState, DiscoveredApplication, DiscoverySource,
};
use deskpilot_lib::state::AppState;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

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
    let count = state
        .update_catalog(apps.clone())
        .expect("Catalog update must succeed");
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
    let temp_exe = create_temp_exe_fixture("valid");
    let id = "app_33333333333333333333333333333333";

    state
        .update_catalog(vec![make_discovered_app(
            id,
            "Launchable App",
            Some(temp_exe.to_string_lossy().to_string()),
            AvailabilityState::Available,
        )])
        .unwrap();

    let validated = ApplicationLauncherService::resolve_and_validate(&state, id)
        .expect("Valid target must pass pre-flight resolution and validation");

    assert_eq!(validated.application_id, id);
    assert_eq!(validated.display_name, "Launchable App");
    assert!(validated.executable_path.is_file());
    assert!(validated.working_directory.is_some());

    let _ = std::fs::remove_file(temp_exe);
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
