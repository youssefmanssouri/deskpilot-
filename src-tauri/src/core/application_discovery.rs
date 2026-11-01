use crate::core::application_identity::ApplicationIdentity;
use crate::error::DomainError;
use crate::models::application::{AvailabilityState, DiscoveredApplication, DiscoverySource};
use crate::platform::RawDiscoveredEntry;
use std::path::Path;

/// Aggregated discovery result from all scanned system sources.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveryResult {
    pub applications: Vec<DiscoveredApplication>,
    pub scanned_sources: Vec<String>,
    pub partial_failures: Vec<String>,
}

/// Core domain service orchestrating Windows application discovery, normalization, and deduplication.
pub struct ApplicationDiscoveryService;

impl ApplicationDiscoveryService {
    /// Executes full application discovery across Windows registry hives and Start Menu directories.
    pub fn discover_all() -> Result<DiscoveryResult, DomainError> {
        let mut raw_entries = Vec::new();
        let mut scanned_sources = Vec::new();
        let mut partial_failures = Vec::new();

        #[cfg(target_os = "windows")]
        {
            use crate::platform::WindowsApplicationScanner;
            use winreg::enums::*;

            // 1. HKLM 64-bit Registry Hive
            scanned_sources.push("Registry (HKLM 64-bit)".to_string());
            match WindowsApplicationScanner::scan_registry_hive(
                HKEY_LOCAL_MACHINE,
                r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
                KEY_READ | KEY_WOW64_64KEY,
                DiscoverySource::RegistryHklm64,
            ) {
                Ok(mut entries) => raw_entries.append(&mut entries),
                Err(err) => partial_failures.push(format!("HKLM 64-bit: {}", err)),
            }

            // 2. HKLM 32-bit (Wow6432Node) Registry Hive
            scanned_sources.push("Registry (HKLM 32-bit)".to_string());
            match WindowsApplicationScanner::scan_registry_hive(
                HKEY_LOCAL_MACHINE,
                r"Software\Wow6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
                KEY_READ | KEY_WOW64_32KEY,
                DiscoverySource::RegistryHklm32,
            ) {
                Ok(mut entries) => raw_entries.append(&mut entries),
                Err(err) => partial_failures.push(format!("HKLM 32-bit: {}", err)),
            }

            // 3. HKCU (Current User) Registry Hive
            scanned_sources.push("Registry (HKCU User)".to_string());
            match WindowsApplicationScanner::scan_registry_hive(
                HKEY_CURRENT_USER,
                r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
                KEY_READ,
                DiscoverySource::RegistryHkcu,
            ) {
                Ok(mut entries) => raw_entries.append(&mut entries),
                Err(err) => partial_failures.push(format!("HKCU User: {}", err)),
            }

            // 4. Common (System-wide) Start Menu Shortcuts
            if let Ok(program_data) = std::env::var("ProgramData") {
                let common_start_menu =
                    Path::new(&program_data).join(r"Microsoft\Windows\Start Menu\Programs");
                scanned_sources.push("Start Menu (Common)".to_string());
                match WindowsApplicationScanner::scan_start_menu_directory(
                    &common_start_menu,
                    DiscoverySource::StartMenuCommon,
                    3,
                ) {
                    Ok(mut entries) => raw_entries.append(&mut entries),
                    Err(err) => partial_failures.push(format!("Start Menu Common: {}", err)),
                }
            }

            // 5. User Start Menu Shortcuts
            if let Ok(app_data) = std::env::var("APPDATA") {
                let user_start_menu =
                    Path::new(&app_data).join(r"Microsoft\Windows\Start Menu\Programs");
                scanned_sources.push("Start Menu (User)".to_string());
                match WindowsApplicationScanner::scan_start_menu_directory(
                    &user_start_menu,
                    DiscoverySource::StartMenuUser,
                    3,
                ) {
                    Ok(mut entries) => raw_entries.append(&mut entries),
                    Err(err) => partial_failures.push(format!("Start Menu User: {}", err)),
                }
            }
        }

        let applications = Self::process_raw_entries(raw_entries);

        Ok(DiscoveryResult {
            applications,
            scanned_sources,
            partial_failures,
        })
    }

    /// Normalizes, deduplicates, and evaluates availability for a collection of raw entries.
    ///
    /// Applies cross-source correlation, prevents false merges between different publishers
    /// or different executable binaries, and calculates a 128-bit collision-resistant identity.
    pub fn process_raw_entries(entries: Vec<RawDiscoveredEntry>) -> Vec<DiscoveredApplication> {
        let mut apps: Vec<DiscoveredApplication> = Vec::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        for entry in entries {
            let raw_name = match entry.raw_display_name {
                Some(name) if !name.trim().is_empty() => name.trim().to_string(),
                _ => continue, // Discard entries without valid display name
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
            let target_basename =
                target_hint.and_then(ApplicationIdentity::extract_target_basename);

            let availability =
                Self::assess_availability(target_path.as_deref(), install_location.as_deref());

            // Search for an existing application representing the same software
            let norm_entry_name = ApplicationIdentity::normalize_display_name(&raw_name);
            let norm_entry_pub = publisher
                .as_deref()
                .and_then(ApplicationIdentity::normalize_publisher);

            let mut matched_idx = None;
            for (idx, existing) in apps.iter().enumerate() {
                // 1. Normalized display names must match
                let norm_exist_name =
                    ApplicationIdentity::normalize_display_name(&existing.display_name);
                if norm_exist_name != norm_entry_name {
                    continue;
                }

                // 2. Publisher compatibility: if both are present, they must normalize to the same value
                let norm_exist_pub = existing
                    .publisher
                    .as_deref()
                    .and_then(ApplicationIdentity::normalize_publisher);
                if let (Some(ep), Some(np)) = (&norm_exist_pub, &norm_entry_pub) {
                    if ep != np {
                        continue; // Different publishers -> distinct software
                    }
                }

                // 3. Executable basename compatibility: if both specify an executable/stem, they must match
                let exist_hint = existing
                    .target_path
                    .as_deref()
                    .or(existing.install_location.as_deref());
                let exist_basename =
                    exist_hint.and_then(ApplicationIdentity::extract_target_basename);
                if let (Some(eb), Some(nb)) = (&exist_basename, &target_basename) {
                    if eb != nb {
                        continue; // Different binaries -> distinct software
                    }
                }

                // 4. Distinct install path compatibility: if both have explicit target paths in different directories, do not merge
                if let (Some(et), Some(nt)) = (&existing.target_path, &target_path) {
                    let ep = Path::new(et);
                    let np = Path::new(nt);
                    if ep != np && ep.parent() != np.parent() {
                        continue; // Different installation directories -> distinct instances
                    }
                }

                // Matches cleanly with no conflicting publisher or binary
                matched_idx = Some(idx);
                break;
            }

            if let Some(idx) = matched_idx {
                let existing = &mut apps[idx];

                // Merge discovery sources
                if !existing.sources.contains(&entry.source) {
                    existing.sources.push(entry.source);
                }

                // Fill missing metadata deterministically
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

                // Merge availability
                existing.availability =
                    Self::merge_availability(existing.availability, availability);

                // Recompute canonical 128-bit identity using enriched metadata
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
                let id =
                    ApplicationIdentity::generate_id(&raw_name, publisher.as_deref(), target_hint);
                apps.push(DiscoveredApplication {
                    id,
                    display_name: raw_name,
                    publisher,
                    version,
                    sources: vec![entry.source],
                    install_location,
                    target_path,
                    availability,
                    discovered_at: now,
                });
            }
        }

        // Sort applications deterministically by display name (case-insensitive), then by ID
        apps.sort_by(|a, b| {
            a.display_name
                .to_lowercase()
                .cmp(&b.display_name.to_lowercase())
                .then_with(|| a.id.cmp(&b.id))
        });

        apps
    }

    /// Assesses whether an application's target is present, missing, unverified, or incomplete.
    pub fn assess_availability(
        target_path: Option<&str>,
        install_location: Option<&str>,
    ) -> AvailabilityState {
        if let Some(target) = target_path {
            if Path::new(target).is_file() {
                AvailabilityState::Available
            } else {
                AvailabilityState::MissingTarget
            }
        } else if let Some(location) = install_location {
            if Path::new(location).is_dir() {
                AvailabilityState::Unverified
            } else {
                AvailabilityState::MissingTarget
            }
        } else {
            AvailabilityState::IncompleteMetadata
        }
    }

    /// Merges two availability states using deterministic precedence:
    /// Available > Unverified > IncompleteMetadata > MissingTarget
    pub fn merge_availability(a: AvailabilityState, b: AvailabilityState) -> AvailabilityState {
        use AvailabilityState::*;
        match (a, b) {
            (Available, _) | (_, Available) => Available,
            (Unverified, _) | (_, Unverified) => Unverified,
            (IncompleteMetadata, _) | (_, IncompleteMetadata) => IncompleteMetadata,
            (MissingTarget, MissingTarget) => MissingTarget,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_raw_entries_deduplication() {
        let entry1 = RawDiscoveredEntry {
            raw_display_name: Some("Google Chrome".to_string()),
            raw_publisher: Some("Google LLC".to_string()),
            raw_version: Some("156.0.0".to_string()),
            raw_install_location: None,
            raw_display_icon: None,
            raw_target_path: Some(
                "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe".to_string(),
            ),
            source: DiscoverySource::RegistryHklm32,
        };

        let entry2 = RawDiscoveredEntry {
            raw_display_name: Some("Google Chrome".to_string()),
            raw_publisher: None, // Start Menu shortcuts don't provide publisher
            raw_version: None,
            raw_install_location: None,
            raw_display_icon: None,
            raw_target_path: Some(
                "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe".to_string(),
            ),
            source: DiscoverySource::StartMenuCommon,
        };

        let results = ApplicationDiscoveryService::process_raw_entries(vec![entry1, entry2]);
        assert_eq!(
            results.len(),
            1,
            "Must merge shortcut and registry entry for the same app"
        );
        let app = &results[0];
        assert_eq!(app.display_name, "Google Chrome");
        assert_eq!(app.publisher, Some("Google LLC".to_string()));
        assert_eq!(app.version, Some("156.0.0".to_string()));
        assert_eq!(app.sources.len(), 2);
        assert!(app.sources.contains(&DiscoverySource::RegistryHklm32));
        assert!(app.sources.contains(&DiscoverySource::StartMenuCommon));
    }

    #[test]
    fn test_process_raw_entries_different_publishers() {
        let entry_a = RawDiscoveredEntry {
            raw_display_name: Some("Terminal".to_string()),
            raw_publisher: Some("Company Alpha".to_string()),
            raw_version: Some("1.0".to_string()),
            raw_install_location: None,
            raw_display_icon: None,
            raw_target_path: None,
            source: DiscoverySource::RegistryHkcu,
        };

        let entry_b = RawDiscoveredEntry {
            raw_display_name: Some("Terminal".to_string()),
            raw_publisher: Some("Company Beta".to_string()),
            raw_version: Some("2.0".to_string()),
            raw_install_location: None,
            raw_display_icon: None,
            raw_target_path: None,
            source: DiscoverySource::RegistryHkcu,
        };

        let results = ApplicationDiscoveryService::process_raw_entries(vec![entry_a, entry_b]);
        assert_eq!(results.len(), 2, "Different publishers must not merge");
        assert_ne!(results[0].id, results[1].id);
    }

    #[test]
    fn test_process_raw_entries_same_publisher_different_executables() {
        let entry_teams = RawDiscoveredEntry {
            raw_display_name: Some("Client".to_string()),
            raw_publisher: Some("Microsoft Corporation".to_string()),
            raw_version: Some("1.0".to_string()),
            raw_install_location: None,
            raw_display_icon: None,
            raw_target_path: Some(r"C:\Apps\Teams\teams.exe".to_string()),
            source: DiscoverySource::RegistryHkcu,
        };

        let entry_word = RawDiscoveredEntry {
            raw_display_name: Some("Client".to_string()),
            raw_publisher: Some("Microsoft Corporation".to_string()),
            raw_version: Some("2.0".to_string()),
            raw_install_location: None,
            raw_display_icon: None,
            raw_target_path: Some(r"C:\Apps\Office\winword.exe".to_string()),
            source: DiscoverySource::RegistryHkcu,
        };

        let results =
            ApplicationDiscoveryService::process_raw_entries(vec![entry_teams, entry_word]);
        assert_eq!(
            results.len(),
            2,
            "Different executables from same publisher must not merge"
        );
        assert_ne!(results[0].id, results[1].id);
    }

    #[test]
    fn test_discards_entries_without_display_name() {
        let invalid = RawDiscoveredEntry {
            raw_display_name: None,
            raw_publisher: Some("Publisher".to_string()),
            raw_version: None,
            raw_install_location: None,
            raw_display_icon: None,
            raw_target_path: None,
            source: DiscoverySource::RegistryHklm64,
        };

        let whitespace_only = RawDiscoveredEntry {
            raw_display_name: Some("   ".to_string()),
            raw_publisher: None,
            raw_version: None,
            raw_install_location: None,
            raw_display_icon: None,
            raw_target_path: None,
            source: DiscoverySource::RegistryHklm64,
        };

        let results =
            ApplicationDiscoveryService::process_raw_entries(vec![invalid, whitespace_only]);
        assert!(results.is_empty());
    }

    #[test]
    fn test_availability_merging_precedence() {
        assert_eq!(
            ApplicationDiscoveryService::merge_availability(
                AvailabilityState::Available,
                AvailabilityState::MissingTarget
            ),
            AvailabilityState::Available
        );
        assert_eq!(
            ApplicationDiscoveryService::merge_availability(
                AvailabilityState::MissingTarget,
                AvailabilityState::Unverified
            ),
            AvailabilityState::Unverified
        );
        assert_eq!(
            ApplicationDiscoveryService::merge_availability(
                AvailabilityState::IncompleteMetadata,
                AvailabilityState::MissingTarget
            ),
            AvailabilityState::IncompleteMetadata
        );
    }
}
