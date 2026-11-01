use crate::core::application_identity::ApplicationIdentity;
use crate::error::DomainError;
use crate::models::application::{AvailabilityState, DiscoveredApplication, DiscoverySource};
use crate::platform::RawDiscoveredEntry;
use std::collections::HashMap;
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

            // 1. HKLM 64-bit Registry
            scanned_sources.push("Registry (HKLM 64-bit)".to_string());
            match WindowsApplicationScanner::scan_registry_hive(
                HKEY_LOCAL_MACHINE,
                r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
                DiscoverySource::RegistryHklm64,
            ) {
                Ok(mut entries) => raw_entries.append(&mut entries),
                Err(err) => partial_failures.push(format!("HKLM 64-bit: {}", err)),
            }

            // 2. HKLM 32-bit (Wow6432Node) Registry
            scanned_sources.push("Registry (HKLM 32-bit)".to_string());
            match WindowsApplicationScanner::scan_registry_hive(
                HKEY_LOCAL_MACHINE,
                r"Software\Wow6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
                DiscoverySource::RegistryHklm32,
            ) {
                Ok(mut entries) => raw_entries.append(&mut entries),
                Err(err) => partial_failures.push(format!("HKLM 32-bit: {}", err)),
            }

            // 3. HKCU (Current User) Registry
            scanned_sources.push("Registry (HKCU User)".to_string());
            match WindowsApplicationScanner::scan_registry_hive(
                HKEY_CURRENT_USER,
                r"Software\Microsoft\Windows\CurrentVersion\Uninstall",
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
    pub fn process_raw_entries(entries: Vec<RawDiscoveredEntry>) -> Vec<DiscoveredApplication> {
        let mut grouped: HashMap<String, DiscoveredApplication> = HashMap::new();
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

            // Compute deterministic application ID
            let id = ApplicationIdentity::generate_id(
                &raw_name,
                publisher.as_deref(),
                target_path.as_deref().or(install_location.as_deref()),
            );

            // Assess availability state for this single observation
            let availability =
                Self::assess_availability(target_path.as_deref(), install_location.as_deref());

            // Deduplicate and merge into map
            if let Some(existing) = grouped.get_mut(&id) {
                // Add discovery source provenance if not already recorded
                if !existing.sources.contains(&entry.source) {
                    existing.sources.push(entry.source.clone());
                }

                // Deterministic metadata conflict resolution:
                // 1. Publisher: prefer non-empty publisher
                if existing.publisher.is_none() && publisher.is_some() {
                    existing.publisher = publisher;
                }

                // 2. Version: prefer non-empty version
                if existing.version.is_none() && version.is_some() {
                    existing.version = version;
                }

                // 3. Install location: prefer non-empty
                if existing.install_location.is_none() && install_location.is_some() {
                    existing.install_location = install_location;
                }

                // 4. Target path: prefer verified target
                if existing.target_path.is_none() && target_path.is_some() {
                    existing.target_path = target_path;
                }

                // 5. Availability: Available > Unverified > IncompleteMetadata > MissingTarget
                existing.availability =
                    Self::merge_availability(existing.availability, availability);
            } else {
                grouped.insert(
                    id.clone(),
                    DiscoveredApplication {
                        id,
                        display_name: raw_name,
                        publisher,
                        version,
                        sources: vec![entry.source],
                        install_location,
                        target_path,
                        availability,
                        discovered_at: now,
                    },
                );
            }
        }

        // Sort applications deterministically by display name (case-insensitive)
        let mut result: Vec<DiscoveredApplication> = grouped.into_values().collect();
        result.sort_by(|a, b| {
            a.display_name
                .to_lowercase()
                .cmp(&b.display_name.to_lowercase())
                .then_with(|| a.id.cmp(&b.id))
        });

        result
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
            raw_target_path: None,
            source: DiscoverySource::RegistryHklm32,
        };

        let entry2 = RawDiscoveredEntry {
            raw_display_name: Some("Google Chrome".to_string()),
            raw_publisher: Some("Google LLC".to_string()),
            raw_version: None,
            raw_install_location: None,
            raw_display_icon: None,
            raw_target_path: None,
            source: DiscoverySource::StartMenuCommon,
        };

        let results = ApplicationDiscoveryService::process_raw_entries(vec![entry1, entry2]);
        assert_eq!(results.len(), 1);
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
        assert_eq!(results.len(), 2);
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
