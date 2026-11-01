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
    /// Formats a clean, sanitized partial failure description that excludes all raw paths,
    /// usernames, environment variable values, and registry keys.
    pub fn format_sanitized_failure(source_label: &str, raw_reason: &str) -> String {
        let sanitized = Self::sanitize_error_text(raw_reason);
        format!("{}: {}", source_label, sanitized)
    }

    /// Strips any raw paths, usernames, or internal operating system references,
    /// returning a stable, user-safe error category.
    pub fn sanitize_error_text(raw: &str) -> String {
        let lower = raw.to_lowercase();
        if lower.contains("access is denied") || lower.contains("permission denied") {
            "Access is denied.".to_string()
        } else if lower.contains("not found") || lower.contains("does not exist") {
            "Resource not found or unavailable.".to_string()
        } else if lower.contains("environment variable") {
            "Required environment variable is not defined.".to_string()
        } else {
            "Subsystem or resource is currently inaccessible.".to_string()
        }
    }

    /// Compares two filesystem paths for equality in a platform-appropriate manner.
    ///
    /// On Windows, compares path components case-insensitively with full Unicode support,
    /// normalizes path separators (`/` vs `\`), and treats drive letters (`c:` vs `C:`) as identical.
    /// On other platforms, performs standard case-sensitive path comparison.
    pub fn paths_equal(path_a: &Path, path_b: &Path) -> bool {
        #[cfg(target_os = "windows")]
        {
            let mut comp_a = path_a.components();
            let mut comp_b = path_b.components();

            loop {
                match (comp_a.next(), comp_b.next()) {
                    (None, None) => return true,
                    (Some(ca), Some(cb)) => {
                        if !Self::components_equal_windows(&ca, &cb) {
                            return false;
                        }
                    }
                    _ => return false,
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            path_a == path_b
        }
    }

    /// Compares the parent directories of two paths for equality in a platform-appropriate manner.
    pub fn path_parents_equal(path_a: &Path, path_b: &Path) -> bool {
        match (path_a.parent(), path_b.parent()) {
            (Some(pa), Some(pb)) => Self::paths_equal(pa, pb),
            (None, None) => true,
            _ => false,
        }
    }

    #[cfg(target_os = "windows")]
    fn components_equal_windows(ca: &std::path::Component, cb: &std::path::Component) -> bool {
        use std::path::{Component, Prefix};

        match (ca, cb) {
            (Component::Prefix(pa), Component::Prefix(pb)) => match (pa.kind(), pb.kind()) {
                (Prefix::Disk(da), Prefix::Disk(db))
                | (Prefix::VerbatimDisk(da), Prefix::VerbatimDisk(db))
                | (Prefix::Disk(da), Prefix::VerbatimDisk(db))
                | (Prefix::VerbatimDisk(da), Prefix::Disk(db)) => {
                    da.to_ascii_uppercase() == db.to_ascii_uppercase()
                }
                (Prefix::UNC(s_a, sh_a), Prefix::UNC(s_b, sh_b))
                | (Prefix::VerbatimUNC(s_a, sh_a), Prefix::VerbatimUNC(s_b, sh_b)) => {
                    Self::str_equal_ignore_case(
                        s_a.to_string_lossy().as_ref(),
                        s_b.to_string_lossy().as_ref(),
                    ) && Self::str_equal_ignore_case(
                        sh_a.to_string_lossy().as_ref(),
                        sh_b.to_string_lossy().as_ref(),
                    )
                }
                (Prefix::DeviceNS(d_a), Prefix::DeviceNS(d_b))
                | (Prefix::Verbatim(d_a), Prefix::Verbatim(d_b)) => Self::str_equal_ignore_case(
                    d_a.to_string_lossy().as_ref(),
                    d_b.to_string_lossy().as_ref(),
                ),
                _ => false,
            },
            (Component::RootDir, Component::RootDir)
            | (Component::CurDir, Component::CurDir)
            | (Component::ParentDir, Component::ParentDir) => true,
            (Component::Normal(oa), Component::Normal(ob)) => Self::str_equal_ignore_case(
                oa.to_string_lossy().as_ref(),
                ob.to_string_lossy().as_ref(),
            ),
            _ => false,
        }
    }

    fn str_equal_ignore_case(a: &str, b: &str) -> bool {
        a.chars()
            .flat_map(|c| c.to_lowercase())
            .eq(b.chars().flat_map(|c| c.to_lowercase()))
    }

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
                Err(err) => partial_failures.push(Self::format_sanitized_failure(
                    "Registry (HKLM 64-bit)",
                    &err,
                )),
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
                Err(err) => partial_failures.push(Self::format_sanitized_failure(
                    "Registry (HKLM 32-bit)",
                    &err,
                )),
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
                Err(err) => partial_failures
                    .push(Self::format_sanitized_failure("Registry (HKCU User)", &err)),
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
                    Err(err) => partial_failures
                        .push(Self::format_sanitized_failure("Start Menu (Common)", &err)),
                }
            } else {
                partial_failures.push(Self::format_sanitized_failure(
                    "Start Menu (Common)",
                    "Environment variable unavailable.",
                ));
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
                    Err(err) => partial_failures
                        .push(Self::format_sanitized_failure("Start Menu (User)", &err)),
                }
            } else {
                partial_failures.push(Self::format_sanitized_failure(
                    "Start Menu (User)",
                    "Environment variable unavailable.",
                ));
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
                // Uses case-insensitive Windows path comparison to prevent case-mismatch false splits
                if let (Some(et), Some(nt)) = (&existing.target_path, &target_path) {
                    let ep = Path::new(et);
                    let np = Path::new(nt);
                    if !Self::paths_equal(ep, np) && !Self::path_parents_equal(ep, np) {
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
    fn test_path_comparison_case_insensitivity() {
        assert!(ApplicationDiscoveryService::paths_equal(
            Path::new(r"C:\Program Files\App\app.exe"),
            Path::new(r"c:\program files\app\app.exe")
        ));
        assert!(ApplicationDiscoveryService::paths_equal(
            Path::new(r"C:/Program Files/App/app.exe"),
            Path::new(r"c:\program files\app\app.exe")
        ));
        assert!(ApplicationDiscoveryService::paths_equal(
            Path::new(r"C:\Programme\ÜberTool.exe"),
            Path::new(r"c:\programme\übertool.exe")
        ));
        assert!(!ApplicationDiscoveryService::paths_equal(
            Path::new(r"C:\App1\app.exe"),
            Path::new(r"C:\App2\app.exe")
        ));
        assert!(ApplicationDiscoveryService::path_parents_equal(
            Path::new(r"C:\Program Files\App\app.exe"),
            Path::new(r"c:\program files\app\other.exe")
        ));
        assert!(!ApplicationDiscoveryService::path_parents_equal(
            Path::new(r"C:\App1\app.exe"),
            Path::new(r"C:\App2\app.exe")
        ));
    }

    #[test]
    fn test_cross_source_merging_different_path_casing() {
        let entry_reg = RawDiscoveredEntry {
            raw_display_name: Some("Visual Studio Code".to_string()),
            raw_publisher: Some("Microsoft Corporation".to_string()),
            raw_version: Some("1.99.1".to_string()),
            raw_install_location: None,
            raw_display_icon: None,
            raw_target_path: Some(r"C:\Program Files\Microsoft VS Code\Code.exe".to_string()),
            source: DiscoverySource::RegistryHkcu,
        };

        let entry_shortcut = RawDiscoveredEntry {
            raw_display_name: Some("Visual Studio Code".to_string()),
            raw_publisher: None,
            raw_version: None,
            raw_install_location: None,
            raw_display_icon: None,
            // Different drive-letter casing and folder casing
            raw_target_path: Some(r"c:\program files\microsoft vs code\code.exe".to_string()),
            source: DiscoverySource::StartMenuUser,
        };

        let results =
            ApplicationDiscoveryService::process_raw_entries(vec![entry_reg, entry_shortcut]);
        assert_eq!(
            results.len(),
            1,
            "Observations with different path casing must merge into a single application"
        );
        assert_eq!(results[0].sources.len(), 2);
    }

    #[test]
    fn test_different_installation_directories_do_not_merge() {
        let entry1 = RawDiscoveredEntry {
            raw_display_name: Some("Python".to_string()),
            raw_publisher: Some("Python Software Foundation".to_string()),
            raw_version: Some("3.11".to_string()),
            raw_install_location: None,
            raw_display_icon: None,
            raw_target_path: Some(r"C:\Python311\python.exe".to_string()),
            source: DiscoverySource::RegistryHklm64,
        };

        let entry2 = RawDiscoveredEntry {
            raw_display_name: Some("Python".to_string()),
            raw_publisher: Some("Python Software Foundation".to_string()),
            raw_version: Some("3.12".to_string()),
            raw_install_location: None,
            raw_display_icon: None,
            raw_target_path: Some(r"C:\Python312\python.exe".to_string()),
            source: DiscoverySource::RegistryHklm64,
        };

        let results = ApplicationDiscoveryService::process_raw_entries(vec![entry1, entry2]);
        assert_eq!(
            results.len(),
            2,
            "Distinct installation directories must remain separate applications"
        );
        assert_ne!(results[0].id, results[1].id);
    }

    #[test]
    fn test_unicode_path_deduplication() {
        let entry1 = RawDiscoveredEntry {
            raw_display_name: Some("Editor".to_string()),
            raw_publisher: None,
            raw_version: None,
            raw_install_location: None,
            raw_display_icon: None,
            raw_target_path: Some(r"C:\Programme\ÜberApp\App.exe".to_string()),
            source: DiscoverySource::RegistryHklm64,
        };

        let entry2 = RawDiscoveredEntry {
            raw_display_name: Some("Editor".to_string()),
            raw_publisher: None,
            raw_version: None,
            raw_install_location: None,
            raw_display_icon: None,
            raw_target_path: Some(r"c:\programme\überapp\app.exe".to_string()),
            source: DiscoverySource::StartMenuCommon,
        };

        let results = ApplicationDiscoveryService::process_raw_entries(vec![entry1, entry2]);
        assert_eq!(
            results.len(),
            1,
            "Unicode paths differing only in casing must merge"
        );
    }

    #[test]
    fn test_partial_failure_sanitization_scrubs_usernames_and_paths() {
        let synthetic_user_err = r#"Inaccessible directory: "C:\Users\JohnDoe\AppData\Roaming\Microsoft\Windows\Start Menu": Access is denied."#;
        let sanitized = ApplicationDiscoveryService::format_sanitized_failure(
            "Start Menu (User)",
            synthetic_user_err,
        );
        assert_eq!(sanitized, "Start Menu (User): Access is denied.");
        assert!(!sanitized.contains("JohnDoe"));
        assert!(!sanitized.contains("Users"));
        assert!(!sanitized.contains(r"C:\"));
        assert!(!sanitized.contains("AppData"));

        let synthetic_registry_err = r#"Inaccessible registry key Software\Microsoft\Windows\CurrentVersion\Uninstall: Not found."#;
        let sanitized_reg = ApplicationDiscoveryService::format_sanitized_failure(
            "Registry (HKLM 64-bit)",
            synthetic_registry_err,
        );
        assert_eq!(
            sanitized_reg,
            "Registry (HKLM 64-bit): Resource not found or unavailable."
        );
        assert!(!sanitized_reg.contains(r"Software\Microsoft"));
        assert!(!sanitized_reg.contains("CurrentVersion"));
        assert!(!sanitized_reg.contains("Uninstall"));
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
