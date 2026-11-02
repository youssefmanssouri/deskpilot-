use crate::error::DomainError;
use crate::models::application::AvailabilityState;
use crate::security::InputGuard;
use crate::state::AppState;
use std::path::{Path, PathBuf};

/// Pre-flight validated application launch target ready for future process execution.
///
/// Contains privileged execution metadata verified by strict pre-flight checks.
/// Raw paths and machine internals are strictly encapsulated and NEVER exposed across IPC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedLaunchTarget {
    pub application_id: String,
    pub display_name: String,
    pub executable_path: PathBuf,
    pub working_directory: Option<PathBuf>,
}

/// Launcher domain service for resolving and pre-validating application launch targets.
///
/// Enforces trust boundaries, identifier formatting, availability state, upfront lexical screening,
/// regular file checks, canonicalization, and post-canonicalization extension verification
/// prior to any process execution.
///
/// # Security & TOCTOU Architecture Documentation
///
/// ## 1. Residual Race Condition (TOCTOU)
/// Pre-flight validation confirms file existence, file type, `.exe` extension, and canonical path
/// at the instant of check. On standard operating systems without transactional filesystem locks,
/// an external process with write permissions could theoretically modify, replace, or delete the
/// target binary in the microsecond interval between validation and process spawning.
/// Canonicalization does NOT eliminate TOCTOU races or establish executable trust.
///
/// ## 2. Reparse-Point and Symbolic-Link Resolution Limitations
/// `std::fs::canonicalize` inspects and resolves intermediate directory junctions, symlinks, and
/// reparse points, returning the final target on disk. DeskPilot strictly validates that the
/// final canonical target also retains a valid `.exe` extension. However, DeskPilot does not inspect
/// complex nested filter drivers or specialized custom reparse tags beyond standard Windows resolution.
///
/// ## 3. Upfront UNC and Device Namespace Screening
/// Executables residing on remote SMB/UNC shares (`\\server\share`, `//server/share`, `\\?\UNC\...`)
/// and Windows device namespaces (`\\.\...`, `\\?\GlobalRoot\...`, `\\?\Volume...`) are strictly
/// rejected BEFORE any filesystem API (`metadata`, `canonicalize`) is called, preventing outbound
/// SMB authentication negotiation, NTLM challenge-response leakage, and network timeout hangs.
///
/// ## 4. Working-Directory and DLL Search-Order Implications
/// DeskPilot sets the process working directory to the target binary's canonical parent folder.
/// This matches standard Windows desktop shortcut semantics. However, setting the working directory
/// to the executable's parent folder does NOT eliminate DLL hijacking risks if the application is installed
/// in a user-writable directory (e.g. `%TEMP%`, `Downloads`, or unprivileged `%LOCALAPPDATA%` folders)
/// where companion DLLs may be placed by unprivileged software.
pub struct ApplicationLauncherService;

impl ApplicationLauncherService {
    /// Resolves an application by its internal ID from the in-memory catalog
    /// and performs comprehensive pre-flight validation on the target executable.
    pub fn resolve_and_validate(
        state: &AppState,
        application_id: &str,
    ) -> Result<ValidatedLaunchTarget, DomainError> {
        // 1. Strictly validate identifier format (`app_[0-9a-f]{32}`)
        InputGuard::validate_application_id(application_id)?;

        // 2. Resolve from trusted in-memory catalog
        // Note: Read lock is acquired and released immediately within `get_application`.
        // We NEVER hold a catalog lock while performing disk I/O or pre-flight checks.
        let app = state.get_application(application_id)?.ok_or_else(|| {
            DomainError::NotFound("Application not found in current catalog.".to_string())
        })?;

        // 3. Reject applications whose availability is not `Available`
        if app.availability != AvailabilityState::Available {
            return Err(DomainError::Unavailable(
                "Application is not in an available state.".to_string(),
            ));
        }

        // 4. Reject records without a target path
        let raw_target = app.target_path.as_deref().ok_or_else(|| {
            DomainError::Unavailable(
                "Application does not specify an executable target path.".to_string(),
            )
        })?;

        // 5. Lexically validate path and extension BEFORE ANY filesystem operation
        Self::validate_lexical_path(raw_target)?;

        let path = Path::new(raw_target);

        // 6. Check that target exists and is a regular file
        let metadata =
            std::fs::metadata(path).map_err(|err| Self::sanitize_io_error(err.kind()))?;

        if !metadata.is_file() {
            return Err(DomainError::InvalidInput(
                "Target path does not point to a regular file.".to_string(),
            ));
        }

        // 7. Canonicalize target path to resolve 8.3 short names and symlinks
        let canonical =
            std::fs::canonicalize(path).map_err(|err| Self::sanitize_io_error(err.kind()))?;

        // 8. Inspect the canonical path's prefix BEFORE any normalization
        Self::validate_canonical_prefix(&canonical)?;

        // 9. Normalize verbatim disk prefix (\\?\C:\) for downstream compatibility
        let clean_path = Self::normalize_verbatim_path(canonical);

        // 10. Re-verify extension strictly on the post-canonicalization target
        Self::validate_target_extension(&clean_path)?;

        // 11. Compute working directory from the validated executable parent folder
        let working_directory = clean_path.parent().map(|p| p.to_path_buf());

        Ok(ValidatedLaunchTarget {
            application_id: app.id,
            display_name: app.display_name,
            executable_path: clean_path,
            working_directory,
        })
    }

    /// Pure lexical screening of raw target path string WITHOUT any filesystem I/O.
    ///
    /// Rejects standard UNC (`\\server\share`, `//server/share`), verbatim UNC (`\\?\UNC\...`),
    /// device namespaces (`\\.\...`), NT namespaces (`\\?\GlobalRoot\...`, `\\?\Volume...`),
    /// and non-`.exe` file extensions.
    pub fn validate_lexical_path(raw_target: &str) -> Result<(), DomainError> {
        let trimmed = raw_target.trim();
        if trimmed.is_empty() {
            return Err(DomainError::InvalidInput(
                "Executable target path cannot be empty.".to_string(),
            ));
        }

        // Rejection of network shares, verbatim UNC, and device namespaces
        if trimmed.starts_with(r"\\")
            || trimmed.starts_with("//")
            || trimmed.starts_with(r"\/")
            || trimmed.starts_with(r"/\")
        {
            if trimmed.starts_with(r"\\?\") || trimmed.starts_with("//?/") {
                let rest = &trimmed[4..];
                let upper = rest.to_ascii_uppercase();
                // Verbatim UNC: \\?\UNC\server\share
                if upper.starts_with("UNC\\") || upper.starts_with("UNC/") {
                    return Err(DomainError::PermissionDenied(
                        "Launching executables from network shares is prohibited.".to_string(),
                    ));
                }
                // Prohibited device/kernel namespaces: Volume, GlobalRoot, pipe, etc.
                if upper.starts_with("VOLUME")
                    || upper.starts_with("GLOBALROOT")
                    || upper.starts_with("PIPE")
                    || upper.starts_with("MAILSLOT")
                {
                    return Err(DomainError::PermissionDenied(
                        "Launching executables from device namespaces is prohibited.".to_string(),
                    ));
                }
                // Check if rest is a valid verbatim disk drive letter, e.g. "C:\"
                if !(rest.len() >= 2
                    && rest.as_bytes()[0].is_ascii_alphabetic()
                    && rest.as_bytes()[1] == b':')
                {
                    return Err(DomainError::PermissionDenied(
                        "Target path uses an unsupported or prohibited namespace.".to_string(),
                    ));
                }
            } else if trimmed.starts_with(r"\\.\") || trimmed.starts_with("//./") {
                // Device namespace \\.\
                return Err(DomainError::PermissionDenied(
                    "Launching executables from device namespaces is prohibited.".to_string(),
                ));
            } else {
                // Standard UNC: \\server\share or //server/share
                return Err(DomainError::PermissionDenied(
                    "Launching executables from network shares is prohibited.".to_string(),
                ));
            }
        }

        // Inspect prefix components via std::path on Windows (pure lexical inspection, no I/O)
        #[cfg(target_os = "windows")]
        {
            use std::path::{Component, Prefix};
            let p = Path::new(raw_target);
            if let Some(Component::Prefix(prefix)) = p.components().next() {
                match prefix.kind() {
                    Prefix::UNC(_, _) | Prefix::VerbatimUNC(_, _) => {
                        return Err(DomainError::PermissionDenied(
                            "Launching executables from network shares is prohibited.".to_string(),
                        ));
                    }
                    Prefix::DeviceNS(_) => {
                        return Err(DomainError::PermissionDenied(
                            "Launching executables from device namespaces is prohibited."
                                .to_string(),
                        ));
                    }
                    Prefix::Verbatim(_) => {
                        return Err(DomainError::PermissionDenied(
                            "Launching executables from prohibited namespace is prohibited."
                                .to_string(),
                        ));
                    }
                    Prefix::Disk(_) | Prefix::VerbatimDisk(_) => {}
                }
            }
        }

        // Raw extension check: must strictly be `.exe` (case-insensitive)
        let path = Path::new(raw_target);
        match path.extension().and_then(|ext| ext.to_str()) {
            Some(ext) if ext.eq_ignore_ascii_case("exe") => {}
            _ => {
                return Err(DomainError::PermissionDenied(
                    "Target file is not a supported executable format.".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Validates canonical path prefix before any normalization is performed.
    pub fn validate_canonical_prefix(canonical: &Path) -> Result<(), DomainError> {
        #[cfg(target_os = "windows")]
        {
            use std::path::{Component, Prefix};
            if let Some(Component::Prefix(prefix)) = canonical.components().next() {
                match prefix.kind() {
                    Prefix::UNC(_, _) | Prefix::VerbatimUNC(_, _) => {
                        return Err(DomainError::PermissionDenied(
                            "Launching executables from network shares is prohibited.".to_string(),
                        ));
                    }
                    Prefix::DeviceNS(_) => {
                        return Err(DomainError::PermissionDenied(
                            "Launching executables from device namespaces is prohibited."
                                .to_string(),
                        ));
                    }
                    Prefix::Verbatim(_) => {
                        return Err(DomainError::PermissionDenied(
                            "Launching executables from prohibited namespace is prohibited."
                                .to_string(),
                        ));
                    }
                    Prefix::Disk(_) | Prefix::VerbatimDisk(_) => {}
                }
            }
        }

        if let Some(s) = canonical.to_str() {
            let upper = s.to_ascii_uppercase();
            if upper.starts_with(r"\\?\UNC\")
                || upper.starts_with("//?/UNC/")
                || upper.starts_with(r"\\.\")
            {
                return Err(DomainError::PermissionDenied(
                    "Launching executables from network shares is prohibited.".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Safely normalizes Windows verbatim disk path (`\\?\C:\...`) to standard drive path (`C:\...`).
    /// Does not strip prefixes on non-disk paths or convert UNC prefixes into relative paths.
    pub fn normalize_verbatim_path(canonical: PathBuf) -> PathBuf {
        match canonical.to_str() {
            Some(s)
                if s.starts_with(r"\\?\")
                    && s.len() >= 6
                    && s.as_bytes()[4].is_ascii_alphabetic()
                    && s.as_bytes()[5] == b':'
                    && (s.as_bytes()[6] == b'\\' || s.as_bytes()[6] == b'/') =>
            {
                PathBuf::from(&s[4..])
            }
            _ => canonical,
        }
    }

    /// Verifies that a path has a valid `.exe` extension (case-insensitive) without panicking on non-Unicode paths.
    pub fn validate_target_extension(path: &Path) -> Result<(), DomainError> {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some(ext) if ext.eq_ignore_ascii_case("exe") => Ok(()),
            _ => Err(DomainError::PermissionDenied(
                "Target file is not a supported executable format.".to_string(),
            )),
        }
    }

    /// Maps standard I/O error kinds to sanitized DomainError categories
    /// without leaking paths, usernames, or internal operating system references.
    pub fn sanitize_io_error(kind: std::io::ErrorKind) -> DomainError {
        match kind {
            std::io::ErrorKind::NotFound => DomainError::Unavailable(
                "Executable target file does not exist on disk.".to_string(),
            ),
            std::io::ErrorKind::PermissionDenied => DomainError::PermissionDenied(
                "Access to executable target file is denied.".to_string(),
            ),
            _ => DomainError::Unavailable("Executable target file is inaccessible.".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::application::{AvailabilityState, DiscoveredApplication, DiscoverySource};
    use std::fs::File;
    use std::io::Write;

    fn create_temp_test_file(name: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let unique_name = format!("deskpilot_test_{}_{}_{}", std::process::id(), nanos, name);
        path.push(unique_name);
        let mut file = File::create(&path).expect("Failed to create test fixture file");
        file.write_all(b"MZ\x00\x00synthetic_executable_binary_content")
            .expect("Failed to write test fixture");
        path
    }

    fn make_app_with_target(
        id: &str,
        target: Option<String>,
        availability: AvailabilityState,
    ) -> DiscoveredApplication {
        DiscoveredApplication {
            id: id.to_string(),
            display_name: "Test App".to_string(),
            publisher: Some("Test Publisher".to_string()),
            version: Some("1.0.0".to_string()),
            sources: vec![DiscoverySource::TestFixture],
            install_location: None,
            target_path: target,
            availability,
            discovered_at: 1000,
        }
    }

    #[test]
    fn test_lexical_path_screening_unc_and_device() {
        // Standard UNC
        assert!(
            ApplicationLauncherService::validate_lexical_path(r"\\server\share\app.exe").is_err()
        );
        assert!(
            ApplicationLauncherService::validate_lexical_path("//server/share/app.exe").is_err()
        );
        assert!(
            ApplicationLauncherService::validate_lexical_path(r"\\192.168.1.1\share\app.exe")
                .is_err()
        );

        // Verbatim UNC
        assert!(
            ApplicationLauncherService::validate_lexical_path(r"\\?\UNC\server\share\app.exe")
                .is_err()
        );
        assert!(
            ApplicationLauncherService::validate_lexical_path("//?/UNC/server/share/app.exe")
                .is_err()
        );

        // Device namespaces
        assert!(ApplicationLauncherService::validate_lexical_path(r"\\.\COM1").is_err());
        assert!(ApplicationLauncherService::validate_lexical_path(r"\\.\PhysicalDrive0").is_err());
        assert!(
            ApplicationLauncherService::validate_lexical_path(r"\\?\Volume{1234}\app.exe").is_err()
        );
        assert!(
            ApplicationLauncherService::validate_lexical_path(
                r"\\?\GlobalRoot\Device\Harddisk0\app.exe"
            )
            .is_err()
        );

        // Valid local drive paths
        assert!(
            ApplicationLauncherService::validate_lexical_path(r"C:\Program Files\App\app.exe")
                .is_ok()
        );
        assert!(
            ApplicationLauncherService::validate_lexical_path(r"\\?\C:\Program Files\App\app.exe")
                .is_ok()
        );

        // Non-exe extensions
        assert!(ApplicationLauncherService::validate_lexical_path(r"C:\App\script.bat").is_err());
        assert!(ApplicationLauncherService::validate_lexical_path(r"C:\App\script.cmd").is_err());
        assert!(ApplicationLauncherService::validate_lexical_path(r"C:\App\script.ps1").is_err());
        assert!(ApplicationLauncherService::validate_lexical_path(r"C:\App\script.vbs").is_err());
        assert!(ApplicationLauncherService::validate_lexical_path(r"C:\App\library.dll").is_err());
        assert!(ApplicationLauncherService::validate_lexical_path("").is_err());
    }

    #[test]
    fn test_launcher_valid_target_resolution() {
        let temp_exe = create_temp_test_file("app.exe");
        let id = "app_0123456789abcdef0123456789abcdef";
        let state = AppState::new("DeskPilot", "0.1.0");

        let app = make_app_with_target(
            id,
            Some(temp_exe.to_string_lossy().to_string()),
            AvailabilityState::Available,
        );
        state.update_catalog(vec![app]).unwrap();

        let target = ApplicationLauncherService::resolve_and_validate(&state, id)
            .expect("Target must resolve successfully");
        assert_eq!(target.application_id, id);
        assert_eq!(target.display_name, "Test App");
        assert!(target.executable_path.is_file());
        assert_eq!(
            target
                .executable_path
                .extension()
                .unwrap()
                .to_str()
                .unwrap()
                .to_lowercase(),
            "exe"
        );
        assert!(target.working_directory.is_some());

        let _ = std::fs::remove_file(temp_exe);
    }

    #[test]
    fn test_launcher_case_insensitive_exe_extension() {
        let temp_exe = create_temp_test_file("app.EXE");
        let id = "app_0123456789abcdef0123456789abcdef";
        let state = AppState::new("DeskPilot", "0.1.0");

        let app = make_app_with_target(
            id,
            Some(temp_exe.to_string_lossy().to_string()),
            AvailabilityState::Available,
        );
        state.update_catalog(vec![app]).unwrap();

        let target = ApplicationLauncherService::resolve_and_validate(&state, id)
            .expect("Must accept uppercase .EXE");
        assert!(target.executable_path.is_file());

        let _ = std::fs::remove_file(temp_exe);
    }

    #[test]
    fn test_launcher_verbatim_local_path_resolution() {
        let temp_exe = create_temp_test_file("verbatim.exe");
        let id = "app_0123456789abcdef0123456789abcdef";
        let state = AppState::new("DeskPilot", "0.1.0");

        // Prefix with \\?\
        let verbatim_path = format!(r"\\?\{}", temp_exe.to_string_lossy());
        let app = make_app_with_target(id, Some(verbatim_path), AvailabilityState::Available);
        state.update_catalog(vec![app]).unwrap();

        let target = ApplicationLauncherService::resolve_and_validate(&state, id)
            .expect("Must resolve verbatim local disk path");
        assert!(target.executable_path.is_file());
        // Verify prefix was normalized cleanly to standard C:\...
        let path_str = target.executable_path.to_string_lossy();
        assert!(!path_str.starts_with(r"\\?\"));

        let _ = std::fs::remove_file(temp_exe);
    }

    #[test]
    fn test_launcher_rejects_unc_before_filesystem_io() {
        let state = AppState::new("DeskPilot", "0.1.0");
        let id = "app_0123456789abcdef0123456789abcdef";

        // Non-existent network host that would hang if contacted via SMB
        let app = make_app_with_target(
            id,
            Some(r"\\192.0.2.1\share\nonexistent.exe".to_string()),
            AvailabilityState::Available,
        );
        state.update_catalog(vec![app]).unwrap();

        let start = std::time::Instant::now();
        let err = ApplicationLauncherService::resolve_and_validate(&state, id).unwrap_err();
        let elapsed = start.elapsed();

        // Must reject instantaneously via lexical check (< 50ms) without network timeout
        assert!(
            elapsed < std::time::Duration::from_millis(100),
            "Rejection must happen without network timeout"
        );
        match err {
            DomainError::PermissionDenied(msg) => {
                assert!(msg.contains("network shares is prohibited"));
                assert!(!msg.contains("192.0.2.1"));
            }
            _ => panic!("Expected PermissionDenied error"),
        }
    }

    #[test]
    fn test_launcher_rejects_verbatim_unc_path() {
        let state = AppState::new("DeskPilot", "0.1.0");
        let id = "app_0123456789abcdef0123456789abcdef";

        let app = make_app_with_target(
            id,
            Some(r"\\?\UNC\server\share\app.exe".to_string()),
            AvailabilityState::Available,
        );
        state.update_catalog(vec![app]).unwrap();

        let err = ApplicationLauncherService::resolve_and_validate(&state, id).unwrap_err();
        match err {
            DomainError::PermissionDenied(msg) => {
                assert!(msg.contains("network shares is prohibited"));
                assert!(!msg.contains("server"));
            }
            _ => panic!("Expected PermissionDenied error"),
        }
    }

    #[test]
    fn test_launcher_malformed_id_rejection() {
        let state = AppState::new("DeskPilot", "0.1.0");

        let err =
            ApplicationLauncherService::resolve_and_validate(&state, "malformed-id").unwrap_err();
        match err {
            DomainError::InvalidInput(msg) => {
                assert!(msg.contains("Invalid application identifier format"));
                assert!(!msg.contains("C:\\"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_launcher_unknown_id_rejection() {
        let state = AppState::new("DeskPilot", "0.1.0");
        let id = "app_11111111111111111111111111111111";

        let err = ApplicationLauncherService::resolve_and_validate(&state, id).unwrap_err();
        match err {
            DomainError::NotFound(msg) => {
                assert!(msg.contains("Application not found in current catalog"));
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_launcher_missing_target_rejection() {
        let id = "app_22222222222222222222222222222222";
        let state = AppState::new("DeskPilot", "0.1.0");
        let app = make_app_with_target(id, None, AvailabilityState::Available);
        state.update_catalog(vec![app]).unwrap();

        let err = ApplicationLauncherService::resolve_and_validate(&state, id).unwrap_err();
        match err {
            DomainError::Unavailable(msg) => {
                assert!(msg.contains("does not specify an executable target path"));
            }
            _ => panic!("Expected Unavailable error"),
        }
    }

    #[test]
    fn test_launcher_unavailable_state_rejection() {
        let temp_exe = create_temp_test_file("unavail.exe");
        let id = "app_33333333333333333333333333333333";
        let state = AppState::new("DeskPilot", "0.1.0");
        let app = make_app_with_target(
            id,
            Some(temp_exe.to_string_lossy().to_string()),
            AvailabilityState::MissingTarget,
        );
        state.update_catalog(vec![app]).unwrap();

        let err = ApplicationLauncherService::resolve_and_validate(&state, id).unwrap_err();
        match err {
            DomainError::Unavailable(msg) => {
                assert!(msg.contains("not in an available state"));
            }
            _ => panic!("Expected Unavailable error"),
        }

        let _ = std::fs::remove_file(temp_exe);
    }

    #[test]
    fn test_launcher_non_exe_extension_rejection() {
        let temp_bat = create_temp_test_file("script.bat");
        let id = "app_44444444444444444444444444444444";
        let state = AppState::new("DeskPilot", "0.1.0");
        let app = make_app_with_target(
            id,
            Some(temp_bat.to_string_lossy().to_string()),
            AvailabilityState::Available,
        );
        state.update_catalog(vec![app]).unwrap();

        let err = ApplicationLauncherService::resolve_and_validate(&state, id).unwrap_err();
        match err {
            DomainError::PermissionDenied(msg) => {
                assert!(msg.contains("not a supported executable format"));
                assert!(!msg.contains("script.bat"));
            }
            _ => panic!("Expected PermissionDenied error"),
        }

        let _ = std::fs::remove_file(temp_bat);
    }

    #[test]
    fn test_launcher_missing_executable_on_disk_rejection() {
        let id = "app_55555555555555555555555555555555";
        let state = AppState::new("DeskPilot", "0.1.0");
        let non_existent = "C:\\DeskPilot_NonExistent_12345\\missing_app.exe";
        let app = make_app_with_target(
            id,
            Some(non_existent.to_string()),
            AvailabilityState::Available,
        );
        state.update_catalog(vec![app]).unwrap();

        let err = ApplicationLauncherService::resolve_and_validate(&state, id).unwrap_err();
        match err {
            DomainError::Unavailable(msg) => {
                assert!(
                    msg.contains("does not exist on disk") || msg.contains("could not be resolved")
                );
                assert!(!msg.contains("C:\\"));
                assert!(!msg.contains("missing_app.exe"));
            }
            _ => panic!("Expected Unavailable error"),
        }
    }

    #[test]
    fn test_launcher_directory_target_rejection() {
        let temp_dir = std::env::temp_dir();
        let dir_with_exe_name = temp_dir.join("deskpilot_dir_target.exe");
        let _ = std::fs::create_dir(&dir_with_exe_name);

        let id = "app_66666666666666666666666666666666";
        let state = AppState::new("DeskPilot", "0.1.0");
        let app = make_app_with_target(
            id,
            Some(dir_with_exe_name.to_string_lossy().to_string()),
            AvailabilityState::Available,
        );
        state.update_catalog(vec![app]).unwrap();

        let err = ApplicationLauncherService::resolve_and_validate(&state, id).unwrap_err();
        match err {
            DomainError::InvalidInput(msg) => {
                assert!(msg.contains("not point to a regular file"));
            }
            _ => panic!("Expected InvalidInput error for directory target"),
        }

        let _ = std::fs::remove_dir(dir_with_exe_name);
    }

    #[test]
    fn test_symlink_pointing_to_non_exe_target() {
        let temp_bat = create_temp_test_file("payload.bat");
        let mut link_exe = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        link_exe.push(format!(
            "deskpilot_symlink_{}_{}.exe",
            std::process::id(),
            nanos
        ));

        // Attempt creation of symlink (may require Developer Mode or elevation on Windows)
        #[cfg(target_os = "windows")]
        let symlink_created = std::os::windows::fs::symlink_file(&temp_bat, &link_exe).is_ok();
        #[cfg(not(target_os = "windows"))]
        let symlink_created = std::os::unix::fs::symlink(&temp_bat, &link_exe).is_ok();

        if symlink_created {
            let id = "app_77777777777777777777777777777777";
            let state = AppState::new("DeskPilot", "0.1.0");
            let app = make_app_with_target(
                id,
                Some(link_exe.to_string_lossy().to_string()),
                AvailabilityState::Available,
            );
            state.update_catalog(vec![app]).unwrap();

            let err = ApplicationLauncherService::resolve_and_validate(&state, id).unwrap_err();
            match err {
                DomainError::PermissionDenied(msg) => {
                    assert!(msg.contains("not a supported executable format"));
                }
                _ => panic!("Expected PermissionDenied for symlink pointing to .bat"),
            }

            let _ = std::fs::remove_file(&link_exe);
        } else {
            eprintln!(
                "SKIPPED: symlink creation requires Developer Mode or SeCreateSymbolicLinkPrivilege"
            );
        }

        let _ = std::fs::remove_file(temp_bat);
    }

    #[test]
    fn test_launcher_error_privacy_no_path_leakage() {
        let sanitized = ApplicationLauncherService::sanitize_io_error(std::io::ErrorKind::NotFound);
        let msg = format!("{}", sanitized);
        assert!(!msg.contains("C:\\"));
        assert!(!msg.contains("/"));
        assert!(!msg.contains("Users"));
        assert!(!msg.contains("manssouri"));

        let denied =
            ApplicationLauncherService::sanitize_io_error(std::io::ErrorKind::PermissionDenied);
        let msg_denied = format!("{}", denied);
        assert!(!msg_denied.contains("C:\\"));
        assert!(!msg_denied.contains("AppData"));
    }
}
