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
/// Enforces trust boundaries, identifier formatting, availability state, extension verification,
/// regular file checks, and canonical path resolution prior to any process execution.
///
/// # Security & TOCTOU Architecture Documentation
///
/// ## 1. Residual Race Condition (TOCTOU)
/// Pre-flight validation confirms file existence, file type, `.exe` extension, and canonical path
/// at the instant of check. On standard operating systems without transactional filesystem locks,
/// an external process with write permissions could theoretically modify, replace, or delete the
/// target binary in the microsecond interval between validation and process spawning.
///
/// **Mitigations:**
/// - In Phase 4.2, process execution will launch the canonicalized path directly using Win32
///   `CreateProcessW` (`std::process::Command::new`), bypassing all shell interpreters.
/// - DeskPilot enforces standard user least privilege and does not silently elevate.
///
/// ## 2. Reparse-Point and Symbolic-Link Resolution
/// Standard library `std::fs::canonicalize` inspects and resolves all intermediate directory junctions,
/// symlinks, and reparse points, returning the real target on disk.
///
/// ## 3. UNC and Network Path Restrictions
/// Executables residing on remote SMB/UNC shares (`\\server\share\...`) are strictly rejected.
/// Network binaries are vulnerable to network tampering, man-in-the-middle attacks, and NTLM reflection.
///
/// ## 4. Working-Directory and DLL-Loading Implications
/// DeskPilot sets the process working directory to the target binary's parent folder.
/// This matches standard Windows shortcut semantics. Future execution layers must ensure that
/// user-writable temporary directories are not used as arbitrary execution hosts.
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

        // 5. Pre-validate path and extension
        let path = Path::new(raw_target);

        // Validate extension is strictly `.exe` (case-insensitively)
        match path.extension().and_then(|ext| ext.to_str()) {
            Some(ext) if ext.eq_ignore_ascii_case("exe") => {}
            _ => {
                return Err(DomainError::PermissionDenied(
                    "Target file is not a supported executable format.".to_string(),
                ));
            }
        }

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

        // Normalize Windows verbatim prefix (\\?\) for clean downstream compatibility
        let clean_path = match canonical.to_str() {
            Some(s) if s.starts_with(r"\\?\") => PathBuf::from(&s[4..]),
            _ => canonical,
        };

        // 8. Reject UNC and remote network paths
        #[cfg(target_os = "windows")]
        {
            use std::path::{Component, Prefix};
            if let Some(Component::Prefix(prefix)) = clean_path.components().next() {
                match prefix.kind() {
                    Prefix::UNC(_, _) | Prefix::VerbatimUNC(_, _) => {
                        return Err(DomainError::PermissionDenied(
                            "Launching executables from remote network shares is prohibited."
                                .to_string(),
                        ));
                    }
                    _ => {}
                }
            }
        }

        // 9. Compute working directory from the validated executable parent folder
        let working_directory = clean_path.parent().map(|p| p.to_path_buf());

        Ok(ValidatedLaunchTarget {
            application_id: app.id,
            display_name: app.display_name,
            executable_path: clean_path,
            working_directory,
        })
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
        // Unique prefix prevents collision across concurrent test runs
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
        // Append .exe to a directory name to verify regular file check catches it
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
