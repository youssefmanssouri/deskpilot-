// Unit tests verifying Windows application discovery, normalization, identity, and conflict resolution.

#[path = "../src/models/application.rs"]
mod application;
#[path = "../src/models/application_dto.rs"]
mod application_dto;
#[path = "../src/core/application_identity.rs"]
mod application_identity;
#[path = "../src/platform/windows/application_discovery.rs"]
mod platform_discovery;

use application::{AvailabilityState, DiscoverySource};
use application_identity::ApplicationIdentity;
use platform_discovery::{RawDiscoveredEntry, WindowsApplicationScanner};
use std::collections::HashMap;
use std::path::Path;

fn process_test_entries(
    entries: Vec<RawDiscoveredEntry>,
) -> Vec<application::DiscoveredApplication> {
    let mut grouped: HashMap<String, application::DiscoveredApplication> = HashMap::new();

    for entry in entries {
        let raw_name = match entry.raw_display_name {
            Some(name) if !name.trim().is_empty() => name.trim().to_string(),
            _ => continue,
        };

        let publisher = entry
            .raw_publisher
            .filter(|p| !p.trim().is_empty())
            .map(|p| p.trim().to_string());
        let version = entry
            .raw_version
            .filter(|v| !v.trim().is_empty())
            .map(|v| v.trim().to_string());
        let install_location = entry
            .raw_install_location
            .filter(|l| !l.trim().is_empty())
            .map(|l| l.trim().to_string());
        let target_path = entry
            .raw_target_path
            .filter(|t| !t.trim().is_empty())
            .map(|t| t.trim().to_string());

        let id = ApplicationIdentity::generate_id(
            &raw_name,
            publisher.as_deref(),
            target_path.as_deref().or(install_location.as_deref()),
        );

        let availability = if let Some(ref target) = target_path {
            if Path::new(target).is_file() {
                AvailabilityState::Available
            } else {
                AvailabilityState::MissingTarget
            }
        } else if let Some(ref loc) = install_location {
            if Path::new(loc).is_dir() {
                AvailabilityState::Unverified
            } else {
                AvailabilityState::MissingTarget
            }
        } else {
            AvailabilityState::IncompleteMetadata
        };

        if let Some(existing) = grouped.get_mut(&id) {
            if !existing.sources.contains(&entry.source) {
                existing.sources.push(entry.source.clone());
            }
            if existing.publisher.is_none() && publisher.is_some() {
                existing.publisher = publisher;
            }
            if existing.version.is_none() && version.is_some() {
                existing.version = version;
            }
            if existing.install_location.is_none() && install_location.is_some() {
                existing.install_location = install_location;
            }
            if existing.target_path.is_none() && target_path.is_some() {
                existing.target_path = target_path;
            }
            existing.availability = match (existing.availability, availability) {
                (AvailabilityState::Available, _) | (_, AvailabilityState::Available) => {
                    AvailabilityState::Available
                }
                (AvailabilityState::Unverified, _) | (_, AvailabilityState::Unverified) => {
                    AvailabilityState::Unverified
                }
                (AvailabilityState::IncompleteMetadata, _)
                | (_, AvailabilityState::IncompleteMetadata) => {
                    AvailabilityState::IncompleteMetadata
                }
                (AvailabilityState::MissingTarget, AvailabilityState::MissingTarget) => {
                    AvailabilityState::MissingTarget
                }
            };
        } else {
            grouped.insert(
                id.clone(),
                application::DiscoveredApplication {
                    id,
                    display_name: raw_name,
                    publisher,
                    version,
                    sources: vec![entry.source],
                    install_location,
                    target_path,
                    availability,
                    discovered_at: 0,
                },
            );
        }
    }

    let mut result: Vec<_> = grouped.into_values().collect();
    result.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
    });
    result
}

#[test]
fn test_identity_determinism_and_stability() {
    let id_a = ApplicationIdentity::generate_id("Google Chrome", Some("Google LLC"), None);
    let id_b = ApplicationIdentity::generate_id("Google Chrome", Some("Google LLC"), None);
    assert_eq!(
        id_a, id_b,
        "IDs must be identical across runs with identical metadata"
    );
}

#[test]
fn test_identity_differentiation_same_name_different_publisher() {
    let id_alpha = ApplicationIdentity::generate_id("Launcher", Some("Alpha Studios"), None);
    let id_beta = ApplicationIdentity::generate_id("Launcher", Some("Beta Games"), None);
    assert_ne!(
        id_alpha, id_beta,
        "Same app name from different publishers must have distinct IDs"
    );
}

#[test]
fn test_deduplication_and_source_provenance() {
    let entry_reg = RawDiscoveredEntry {
        raw_display_name: Some("Visual Studio Code".to_string()),
        raw_publisher: Some("Microsoft Corporation".to_string()),
        raw_version: Some("1.99.1".to_string()),
        raw_install_location: None,
        raw_display_icon: None,
        raw_target_path: None,
        source: DiscoverySource::RegistryHkcu,
    };

    let entry_shortcut = RawDiscoveredEntry {
        raw_display_name: Some("Visual Studio Code".to_string()),
        raw_publisher: Some("Microsoft Corporation".to_string()),
        raw_version: None,
        raw_install_location: None,
        raw_display_icon: None,
        raw_target_path: None,
        source: DiscoverySource::StartMenuUser,
    };

    let apps = process_test_entries(vec![entry_reg, entry_shortcut]);
    assert_eq!(
        apps.len(),
        1,
        "Duplicate observations must be deduplicated into a single entry"
    );
    assert_eq!(
        apps[0].sources.len(),
        2,
        "Source provenance must record both sources"
    );
    assert!(apps[0].sources.contains(&DiscoverySource::RegistryHkcu));
    assert!(apps[0].sources.contains(&DiscoverySource::StartMenuUser));
    assert_eq!(
        apps[0].version,
        Some("1.99.1".to_string()),
        "Version from Registry must be preserved"
    );
}

#[test]
fn test_missing_and_malformed_metadata_handling() {
    let missing_name = RawDiscoveredEntry {
        raw_display_name: None,
        raw_publisher: Some("Ghost Publisher".to_string()),
        raw_version: None,
        raw_install_location: None,
        raw_display_icon: None,
        raw_target_path: None,
        source: DiscoverySource::RegistryHklm64,
    };

    let empty_name = RawDiscoveredEntry {
        raw_display_name: Some("     ".to_string()),
        raw_publisher: None,
        raw_version: None,
        raw_install_location: None,
        raw_display_icon: None,
        raw_target_path: None,
        source: DiscoverySource::RegistryHklm64,
    };

    let valid = RawDiscoveredEntry {
        raw_display_name: Some("Real Application".to_string()),
        raw_publisher: None,
        raw_version: None,
        raw_install_location: None,
        raw_display_icon: None,
        raw_target_path: None,
        source: DiscoverySource::RegistryHklm64,
    };

    let apps = process_test_entries(vec![missing_name, empty_name, valid]);
    assert_eq!(apps.len(), 1);
    assert_eq!(apps[0].display_name, "Real Application");
}

#[test]
fn test_stale_target_handling() {
    let missing_target_entry = RawDiscoveredEntry {
        raw_display_name: Some("Old App".to_string()),
        raw_publisher: None,
        raw_version: None,
        raw_install_location: None,
        raw_display_icon: None,
        raw_target_path: Some("C:\\NonExistentPath\\deleted_app.exe".to_string()),
        source: DiscoverySource::RegistryHklm64,
    };

    let apps = process_test_entries(vec![missing_target_entry]);
    assert_eq!(apps.len(), 1);
    assert_eq!(apps[0].availability, AvailabilityState::MissingTarget);
}

#[test]
fn test_uninstaller_shortcut_filtering() {
    let uninstaller = Path::new("C:\\ProgramData\\Start\\Uninstall Cyberpunk.lnk");
    assert!(
        WindowsApplicationScanner::inspect_shortcut(uninstaller, &DiscoverySource::StartMenuCommon)
            .is_none()
    );

    let normal = Path::new("C:\\ProgramData\\Start\\Cyberpunk 2077.lnk");
    assert!(
        WindowsApplicationScanner::inspect_shortcut(normal, &DiscoverySource::StartMenuCommon)
            .is_some()
    );
}
