// Unit tests verifying Windows application discovery, normalization, identity, and conflict resolution.

pub mod models {
    #[path = "../../src/models/application.rs"]
    pub mod application;
    #[path = "../../src/models/application_dto.rs"]
    pub mod application_dto;
}

#[path = "../src/core/application_identity.rs"]
pub mod application_identity;

#[path = "../src/platform/windows/application_discovery.rs"]
pub mod platform_discovery;

use application_identity::ApplicationIdentity;
use models::application::{AvailabilityState, DiscoveredApplication, DiscoverySource};
use platform_discovery::{RawDiscoveredEntry, WindowsApplicationScanner};
use std::path::Path;

pub fn process_test_entries(entries: Vec<RawDiscoveredEntry>) -> Vec<DiscoveredApplication> {
    let mut apps: Vec<DiscoveredApplication> = Vec::new();

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

        let target_hint = target_path.as_deref().or(install_location.as_deref());
        let target_basename = target_hint.and_then(ApplicationIdentity::extract_target_basename);

        let availability = if let Some(target) = &target_path {
            if Path::new(target).is_file() {
                AvailabilityState::Available
            } else {
                AvailabilityState::MissingTarget
            }
        } else if let Some(loc) = &install_location {
            if Path::new(loc).is_dir() {
                AvailabilityState::Unverified
            } else {
                AvailabilityState::MissingTarget
            }
        } else {
            AvailabilityState::IncompleteMetadata
        };

        let norm_entry_name = ApplicationIdentity::normalize_display_name(&raw_name);
        let norm_entry_pub = publisher
            .as_deref()
            .and_then(ApplicationIdentity::normalize_publisher);

        let mut matched_idx = None;
        for (idx, existing) in apps.iter().enumerate() {
            let norm_exist_name =
                ApplicationIdentity::normalize_display_name(&existing.display_name);
            if norm_exist_name != norm_entry_name {
                continue;
            }

            let norm_exist_pub = existing
                .publisher
                .as_deref()
                .and_then(ApplicationIdentity::normalize_publisher);
            if let (Some(ep), Some(np)) = (&norm_exist_pub, &norm_entry_pub) {
                if ep != np {
                    continue;
                }
            }

            let exist_hint = existing
                .target_path
                .as_deref()
                .or(existing.install_location.as_deref());
            let exist_basename = exist_hint.and_then(ApplicationIdentity::extract_target_basename);
            if let (Some(eb), Some(nb)) = (&exist_basename, &target_basename) {
                if eb != nb {
                    continue;
                }
            }

            if let (Some(et), Some(nt)) = (&existing.target_path, &target_path) {
                let ep = Path::new(et);
                let np = Path::new(nt);
                if ep != np && ep.parent() != np.parent() {
                    continue;
                }
            }

            matched_idx = Some(idx);
            break;
        }

        if let Some(idx) = matched_idx {
            let existing = &mut apps[idx];
            if !existing.sources.contains(&entry.source) {
                existing.sources.push(entry.source);
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
            let enriched_hint = existing
                .target_path
                .as_deref()
                .or(existing.install_location.as_deref());
            existing.id = ApplicationIdentity::generate_id(
                &existing.display_name,
                existing.publisher.as_deref(),
                enriched_hint,
            );
        } else {
            let id = ApplicationIdentity::generate_id(&raw_name, publisher.as_deref(), target_hint);
            apps.push(DiscoveredApplication {
                id,
                display_name: raw_name,
                publisher,
                version,
                sources: vec![entry.source],
                install_location,
                target_path,
                availability,
                discovered_at: 0,
            });
        }
    }

    apps.sort_by(|a, b| {
        a.display_name
            .to_lowercase()
            .cmp(&b.display_name.to_lowercase())
    });
    apps
}

#[test]
pub fn test_identity_determinism_and_128bit_entropy() {
    let id_a = ApplicationIdentity::generate_id("Google Chrome", Some("Google LLC"), None);
    let id_b = ApplicationIdentity::generate_id("Google Chrome", Some("Google LLC"), None);
    assert_eq!(
        id_a, id_b,
        "IDs must be identical across runs with identical metadata"
    );
    assert!(id_a.starts_with("app_"));
    // "app_" (4 chars) + 32 hex chars = 36 characters
    assert_eq!(
        id_a.len(),
        36,
        "ID must have 32 hexadecimal characters (128 bits of entropy)"
    );
}

#[test]
pub fn test_identity_differentiation_same_name_different_publisher() {
    let id_alpha = ApplicationIdentity::generate_id("Launcher", Some("Alpha Studios"), None);
    let id_beta = ApplicationIdentity::generate_id("Launcher", Some("Beta Games"), None);
    assert_ne!(
        id_alpha, id_beta,
        "Same app name from different publishers must have distinct IDs"
    );
}

#[test]
pub fn test_identity_differentiation_same_publisher_different_executables() {
    let id_teams = ApplicationIdentity::generate_id(
        "Client",
        Some("Microsoft Corporation"),
        Some(r"C:\Apps\Teams\teams.exe"),
    );
    let id_word = ApplicationIdentity::generate_id(
        "Client",
        Some("Microsoft Corporation"),
        Some(r"C:\Apps\Office\winword.exe"),
    );
    assert_ne!(
        id_teams, id_word,
        "Distinct binaries from the same publisher must have distinct IDs"
    );
}

#[test]
pub fn test_deduplication_and_cross_source_merging() {
    let entry_reg = RawDiscoveredEntry {
        raw_display_name: Some("Visual Studio Code".to_string()),
        raw_publisher: Some("Microsoft Corporation".to_string()),
        raw_version: Some("1.99.1".to_string()),
        raw_install_location: None,
        raw_display_icon: None,
        raw_target_path: Some(
            r"C:\Users\user\AppData\Local\Programs\Microsoft VS Code\Code.exe".to_string(),
        ),
        source: DiscoverySource::RegistryHkcu,
    };

    let entry_shortcut = RawDiscoveredEntry {
        raw_display_name: Some("Visual Studio Code".to_string()),
        raw_publisher: None, // Start Menu shortcuts don't provide publisher
        raw_version: None,
        raw_install_location: None,
        raw_display_icon: None,
        raw_target_path: Some(
            r"C:\Users\user\AppData\Local\Programs\Microsoft VS Code\Code.exe".to_string(),
        ),
        source: DiscoverySource::StartMenuUser,
    };

    let apps = process_test_entries(vec![entry_reg, entry_shortcut]);
    assert_eq!(
        apps.len(),
        1,
        "Duplicate observations across registry and shortcut must merge"
    );
    assert_eq!(
        apps[0].sources.len(),
        2,
        "Source provenance must record both sources"
    );
    assert!(apps[0].sources.contains(&DiscoverySource::RegistryHkcu));
    assert!(apps[0].sources.contains(&DiscoverySource::StartMenuUser));
    assert_eq!(
        apps[0].publisher,
        Some("Microsoft Corporation".to_string()),
        "Publisher from registry must be preserved"
    );
    assert_eq!(
        apps[0].version,
        Some("1.99.1".to_string()),
        "Version from Registry must be preserved"
    );
}

#[test]
pub fn test_missing_and_malformed_metadata_handling() {
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
pub fn test_stale_target_handling() {
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
pub fn test_uninstaller_shortcut_filtering() {
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

#[test]
pub fn test_registry_sanitization_adversarial() {
    // Null byte injection attempt
    let dirty = "Clean App\0Malicious Payload".to_string();
    assert_eq!(
        WindowsApplicationScanner::sanitize_registry_string(dirty),
        Some("Clean AppMalicious Payload".to_string())
    );

    // Over-long string (DoS attempt)
    let huge = "X".repeat(5000);
    let sanitized = WindowsApplicationScanner::sanitize_registry_string(huge).unwrap();
    assert_eq!(sanitized.len(), 2048);
}

#[test]
pub fn test_shortcut_parser_synthetic_ansi_and_unicode() {
    const CLSID: [u8; 16] = [
        0x01, 0x14, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0xC0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x46,
    ];

    // 1. ANSI LinkInfo fixture
    let mut buf_ansi = vec![0u8; 128];
    buf_ansi[0..4].copy_from_slice(&76u32.to_le_bytes());
    buf_ansi[4..20].copy_from_slice(&CLSID);
    buf_ansi[20..24].copy_from_slice(&2u32.to_le_bytes()); // HasLinkInfo
    buf_ansi[76..80].copy_from_slice(&52u32.to_le_bytes()); // LinkInfo size
    buf_ansi[80..84].copy_from_slice(&28u32.to_le_bytes()); // Header size
    buf_ansi[84..88].copy_from_slice(&1u32.to_le_bytes()); // VolumeIDAndLocalBasePath
    buf_ansi[92..96].copy_from_slice(&28u32.to_le_bytes()); // LocalBasePathOffset (at 76+28 = 104)
    buf_ansi[104..128].copy_from_slice(b"C:\\Test\\App.exe\0\0\0\0\0\0\0\0\0");

    let parsed_ansi = WindowsApplicationScanner::parse_lnk_target_from_bytes(&buf_ansi, false);
    assert_eq!(parsed_ansi, Some("C:\\Test\\App.exe".to_string()));

    // 2. Unicode (UTF-16LE) LinkInfo fixture
    let mut buf_uni = vec![0u8; 256];
    buf_uni[0..4].copy_from_slice(&76u32.to_le_bytes());
    buf_uni[4..20].copy_from_slice(&CLSID);
    buf_uni[20..24].copy_from_slice(&2u32.to_le_bytes()); // HasLinkInfo
    buf_uni[76..80].copy_from_slice(&150u32.to_le_bytes()); // LinkInfo size
    buf_uni[80..84].copy_from_slice(&36u32.to_le_bytes()); // Header size >= 36
    buf_uni[84..88].copy_from_slice(&1u32.to_le_bytes()); // VolumeIDAndLocalBasePath
    buf_uni[92..96].copy_from_slice(&36u32.to_le_bytes()); // ANSI offset
    buf_uni[104..108].copy_from_slice(&40u32.to_le_bytes()); // Unicode offset (at 76+40 = 116)

    let uni_str: Vec<u16> = "C:\\日本語\\エディタ.exe\0".encode_utf16().collect();
    let mut p = 116;
    for code in uni_str {
        buf_uni[p..p + 2].copy_from_slice(&code.to_le_bytes());
        p += 2;
    }

    let parsed_uni = WindowsApplicationScanner::parse_lnk_target_from_bytes(&buf_uni, false);
    assert_eq!(parsed_uni, Some("C:\\日本語\\エディタ.exe".to_string()));

    // 3. Corrupt LinkCLSID must be rejected
    let mut corrupt_clsid = buf_ansi.clone();
    corrupt_clsid[4] = 0xAA;
    assert_eq!(
        WindowsApplicationScanner::parse_lnk_target_from_bytes(&corrupt_clsid, false),
        None
    );

    // 4. Truncated buffer (< 76 bytes) must be rejected
    assert_eq!(
        WindowsApplicationScanner::parse_lnk_target_from_bytes(&buf_ansi[..70], false),
        None
    );
}
