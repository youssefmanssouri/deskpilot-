use crate::error::DomainError;
use crate::models::LaunchResultDto;
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

/// Result of a successful native process spawn operation.
///
/// Contains the application ID and the assigned operating system Process ID (PID).
///
/// # Operational Distinction
/// Successful process creation confirms only that the operating system accepted
/// the process creation request (`CreateProcessW` / `Command::spawn`) and allocated
/// an initial process object and main thread.
/// It does NOT guarantee that the application completed internal initialization,
/// displayed its graphical user interface, or will not exit prematurely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchResult {
    pub application_id: String,
    pub process_id: u32,
}

impl From<LaunchResult> for LaunchResultDto {
    fn from(result: LaunchResult) -> Self {
        Self {
            application_id: result.application_id,
            process_id: result.process_id,
        }
    }
}

/// Launcher domain service for resolving, pre-validating, and natively launching applications.
///
/// Enforces trust boundaries, identifier formatting, availability state, upfront lexical screening,
/// regular file checks, canonicalization, post-canonicalization extension verification,
/// transient directory exclusion, and direct native execution without shell invocation.
///
/// # Security & TOCTOU Architecture Documentation
///
/// ## 1. Residual Race Condition (TOCTOU)
/// Pre-flight validation confirms file existence, file type, `.exe` extension, execution policy,
/// and canonical path at the exact instant of check.
/// On standard operating systems without transactional filesystem locks, an external process
/// with write permissions to the binary or its parent directory could theoretically modify,
/// replace, or delete the target binary in the window between validation, canonicalization,
/// and process spawning.
/// Canonicalization does NOT eliminate TOCTOU races or guarantee that the binary remains
/// unchanged at the instant of process execution.
///
/// ## 2. Reparse-Point and Symbolic-Link Resolution Limitations
/// `std::fs::canonicalize` inspects and resolves intermediate directory junctions, symlinks, and
/// reparse points, returning the final disk target. DeskPilot strictly validates that the final
/// canonical target also retains a valid `.exe` extension and does not cross into remote UNC
/// or device namespaces. However, DeskPilot does not inspect complex nested filter drivers
/// or specialized custom reparse tags beyond standard Windows Win32 resolution.
///
/// ## 3. Upfront UNC and Device Namespace Screening
/// Executables residing on remote SMB/UNC shares (`\\server\share`, `//server/share`, `\\?\UNC\...`)
/// and Windows device namespaces (`\\.\...`, `\\?\GlobalRoot\...`, `\\?\Volume...`, `\\?\PIPE\...`,
/// `\\?\MAILSLOT\...`) are strictly rejected BEFORE any filesystem API (`metadata`, `canonicalize`)
/// is called, preventing outbound SMB authentication negotiation, NTLM challenge-response leakage,
/// and network timeout hangs.
///
/// ## 4. Working-Directory and DLL Search-Order Implications
/// DeskPilot sets the child process working directory to the target binary's canonical parent
/// folder, matching standard Windows desktop shortcut semantics. However, setting the working
/// directory to the executable's parent folder does NOT eliminate DLL hijacking risks if the
/// application is installed in a user-writable directory where companion DLLs may be placed
/// by unprivileged software.
///
/// ## 5. Transient & User-Writable Directory Policy
/// DeskPilot enforces an explicit policy rejecting executables located in clearly transient,
/// temporary, or untrusted download locations:
/// - Temporary directories: `std::env::temp_dir()`, `%TEMP%`, `%TMP%`, `C:\Windows\Temp`,
///   `*\AppData\Local\Temp\*`, and `*\Temp\*`.
/// - Downloads directories: `%USERPROFILE%\Downloads`, `*\Downloads\*`.
/// - Browser cache directories: `*\INetCache\*`.
///
/// ## 6. Direct Native Execution, Environment Inheritance, and Zero Caller Arguments
/// Processes are created directly via `std::process::Command::new` without shells (`cmd.exe`,
/// `powershell.exe`). The child inherits the standard parent environment without caller-controlled
/// variables or PATH injections. Zero caller arguments are accepted in Phase 4.2.1.
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

        // 11. Enforce user-writable and transient directory policy
        Self::validate_execution_policy(&clean_path)?;

        // 12. Verify target consistency between raw target and canonical target
        Self::validate_target_consistency(raw_target, &clean_path)?;

        // 13. Compute working directory from the validated executable parent folder
        let working_directory = clean_path.parent().map(|p| p.to_path_buf());

        Ok(ValidatedLaunchTarget {
            application_id: app.id,
            display_name: app.display_name,
            executable_path: clean_path,
            working_directory,
        })
    }

    /// Resolves an application from the in-memory catalog and launches it natively.
    ///
    /// # Concurrency & Security Flow:
    /// 1. Validates application ID format strictly (`app_[0-9a-f]{32}`).
    /// 2. Acquires in-memory catalog read lock, clones internal record, and immediately releases lock.
    ///    The catalog lock is NEVER held during filesystem I/O, canonicalization, or process creation.
    ///    Subsequent catalog rediscovery does not retroactively cancel the launch.
    /// 3. Validates availability state is `Available`.
    /// 4. Performs immediate pre-launch validation:
    ///    - Upfront pure-lexical screening (UNC, device namespaces, etc.)
    ///    - Filesystem existence and regular file check
    ///    - Path canonicalization and verbatim prefix normalization
    ///    - Canonical prefix inspection (prevent symlink network hops)
    ///    - Post-canonicalization `.exe` extension re-verification
    ///    - Transient/user-writable directory policy validation
    ///    - Catalog target consistency verification
    /// 5. Derives working directory strictly from canonical executable parent.
    /// 6. Spawns child process directly via `execute_target`.
    pub fn launch(state: &AppState, application_id: &str) -> Result<LaunchResult, DomainError> {
        let target = Self::resolve_and_validate(state, application_id)?;
        Self::execute_target(&target)
    }

    /// Native child process execution directly via `std::process::Command`.
    ///
    /// # Security & Architectural Boundaries:
    /// - Executable path is supplied directly as the program argument to `Command::new`.
    /// - Shell execution (`cmd.exe`, `powershell.exe`, shell strings) is strictly prohibited.
    /// - Working directory is set strictly to the canonical parent folder.
    /// - Zero caller arguments: no command-line arguments are accepted or forwarded.
    /// - Zero environment manipulation: normal process environment is inherited without
    ///   caller overrides or untrusted `PATH` injections.
    /// - Non-blocking: spawns child process without awaiting exit.
    pub fn execute_target(target: &ValidatedLaunchTarget) -> Result<LaunchResult, DomainError> {
        // 1. Re-validate target extension post-canonicalization
        Self::validate_target_extension(&target.executable_path)?;

        // 2. Re-enforce transient / user-writable directory policy
        Self::validate_execution_policy(&target.executable_path)?;

        // 3. Confirm target exists and is a regular file
        let meta = std::fs::metadata(&target.executable_path)
            .map_err(|err| Self::sanitize_io_error(err.kind()))?;
        if !meta.is_file() {
            return Err(DomainError::InvalidInput(
                "Target path does not point to a regular file.".to_string(),
            ));
        }

        // 4. Working directory must be the canonical parent folder
        let working_dir = target.working_directory.as_deref().ok_or_else(|| {
            DomainError::InvalidInput(
                "Target executable does not specify a working directory.".to_string(),
            )
        })?;

        // 5. Construct direct native command without shell expansion
        let mut cmd = std::process::Command::new(&target.executable_path);
        cmd.current_dir(working_dir);

        // 6. Spawn process directly
        let child = cmd.spawn().map_err(Self::sanitize_spawn_error)?;
        let process_id = child.id();

        Ok(LaunchResult {
            application_id: target.application_id.clone(),
            process_id,
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

    /// Maps child process spawn errors to sanitized domain error categories,
    /// ensuring no executable paths, usernames, or operating system command details leak.
    pub fn sanitize_spawn_error(err: std::io::Error) -> DomainError {
        const ERROR_ELEVATION_REQUIRED: i32 = 740;
        if err.raw_os_error() == Some(ERROR_ELEVATION_REQUIRED) {
            return DomainError::PermissionDenied(
                "This application requires administrator privileges (elevation). DeskPilot launches applications with standard user privileges.".to_string(),
            );
        }

        match err.kind() {
            std::io::ErrorKind::NotFound => DomainError::Unavailable(
                "Executable target file does not exist on disk.".to_string(),
            ),
            std::io::ErrorKind::PermissionDenied => DomainError::PermissionDenied(
                "Access to execute target binary was denied by the operating system.".to_string(),
            ),
            _ => DomainError::OperationFailed(
                "Failed to spawn process for launch target.".to_string(),
            ),
        }
    }

    /// Enforces the user-writable and transient directory execution policy.
    ///
    /// Evaluates the canonical executable location against known transient, temporary,
    /// and untrusted user-controlled directories on Windows:
    /// - `%TEMP%` and `%TMP%` environment variables
    /// - `std::env::temp_dir()`
    /// - User Downloads folder (`%USERPROFILE%\Downloads`)
    /// - Windows temporary folders (`\AppData\Local\Temp\`, `\Windows\Temp\`, `\Temp\`)
    /// - Temporary Internet Files / Browser caches (`\INetCache\`)
    ///
    /// Rejects matching paths with a generic `DomainError::PermissionDenied` that
    /// does NOT leak the path, username, or directory structure.
    pub fn validate_execution_policy(canonical_path: &Path) -> Result<(), DomainError> {
        let path_str = match canonical_path.to_str() {
            Some(s) => s.to_lowercase(),
            None => {
                return Err(DomainError::PermissionDenied(
                    "Target path contains invalid non-Unicode characters.".to_string(),
                ));
            }
        };

        // 1. Check against runtime temp_dir
        if let Ok(temp_dir) = std::fs::canonicalize(std::env::temp_dir()) {
            let normalized_temp = Self::normalize_verbatim_path(temp_dir);
            if let Some(temp_str) = normalized_temp.to_str() {
                let temp_lower = temp_str.to_lowercase();
                if path_str.starts_with(&temp_lower) {
                    return Err(DomainError::PermissionDenied(
                        "Launching executables from temporary or transient directories is prohibited.".to_string(),
                    ));
                }
            }
        }

        // 2. Check %TEMP% and %TMP% environment variables
        for var in &["TEMP", "TMP"] {
            if let Ok(val) = std::env::var(var) {
                if let Ok(can) = std::fs::canonicalize(&val) {
                    let norm = Self::normalize_verbatim_path(can);
                    if let Some(s) = norm.to_str() {
                        let s_lower = s.to_lowercase();
                        if path_str.starts_with(&s_lower) {
                            return Err(DomainError::PermissionDenied(
                                "Launching executables from temporary or transient directories is prohibited.".to_string(),
                            ));
                        }
                    }
                }
            }
        }

        // 3. Check %USERPROFILE%\Downloads
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            let downloads = PathBuf::from(&userprofile).join("Downloads");
            if let Ok(can) = std::fs::canonicalize(&downloads) {
                let norm = Self::normalize_verbatim_path(can);
                if let Some(s) = norm.to_str() {
                    let s_lower = s.to_lowercase();
                    if path_str.starts_with(&s_lower) {
                        return Err(DomainError::PermissionDenied(
                            "Launching executables from temporary or transient directories is prohibited.".to_string(),
                        ));
                    }
                }
            }
        }

        // 4. Structural / pattern checks for transient directory names
        if path_str.contains(r"\appdata\local\temp\")
            || path_str.contains("/appdata/local/temp/")
            || path_str.contains(r"\windows\temp\")
            || path_str.contains("/windows/temp/")
            || path_str.contains(r"\temp\")
            || path_str.contains("/temp/")
            || path_str.contains(r"\downloads\")
            || path_str.contains("/downloads/")
            || path_str.contains(r"\inetcache\")
            || path_str.contains("/inetcache/")
        {
            return Err(DomainError::PermissionDenied(
                "Launching executables from temporary or transient directories is prohibited."
                    .to_string(),
            ));
        }

        Ok(())
    }

    /// Confirms that the canonical executable corresponds to the catalog target.
    pub fn validate_target_consistency(
        raw_target: &str,
        clean_path: &Path,
    ) -> Result<(), DomainError> {
        let re_canonical =
            std::fs::canonicalize(raw_target).map_err(|err| Self::sanitize_io_error(err.kind()))?;
        let normalized = Self::normalize_verbatim_path(re_canonical);
        if normalized != clean_path {
            return Err(DomainError::PermissionDenied(
                "Canonical target does not correspond to the registered catalog target."
                    .to_string(),
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::application::{AvailabilityState, DiscoveredApplication, DiscoverySource};
    use std::fs::File;
    use std::io::Write;

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
        let sys_exe = get_test_system_executable();
        let id = "app_0123456789abcdef0123456789abcdef";
        let state = AppState::new("DeskPilot", "0.1.0");

        let app = make_app_with_target(
            id,
            Some(sys_exe.to_string_lossy().to_string()),
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
    }

    #[test]
    fn test_launcher_case_insensitive_exe_extension() {
        let sys_exe = get_test_system_executable();
        let upper_path = sys_exe.to_string_lossy().to_uppercase();
        let id = "app_0123456789abcdef0123456789abcdef";
        let state = AppState::new("DeskPilot", "0.1.0");

        let app = make_app_with_target(id, Some(upper_path), AvailabilityState::Available);
        state.update_catalog(vec![app]).unwrap();

        let target = ApplicationLauncherService::resolve_and_validate(&state, id)
            .expect("Must accept uppercase .EXE");
        assert!(target.executable_path.is_file());
    }

    #[test]
    fn test_launcher_verbatim_local_path_resolution() {
        let sys_exe = get_test_system_executable();
        let id = "app_0123456789abcdef0123456789abcdef";
        let state = AppState::new("DeskPilot", "0.1.0");

        // Prefix with \\?\
        let verbatim_path = format!(r"\\?\{}", sys_exe.to_string_lossy());
        let app = make_app_with_target(id, Some(verbatim_path), AvailabilityState::Available);
        state.update_catalog(vec![app]).unwrap();

        let target = ApplicationLauncherService::resolve_and_validate(&state, id)
            .expect("Must resolve verbatim local disk path");
        assert!(target.executable_path.is_file());
        // Verify prefix was normalized cleanly to standard C:\...
        let path_str = target.executable_path.to_string_lossy();
        assert!(!path_str.starts_with(r"\\?\"));
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
        let id = "app_33333333333333333333333333333333";
        let state = AppState::new("DeskPilot", "0.1.0");
        let app = make_app_with_target(
            id,
            Some("C:\\App\\app.exe".to_string()),
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
    }

    #[test]
    fn test_launcher_non_exe_extension_rejection() {
        let id = "app_44444444444444444444444444444444";
        let state = AppState::new("DeskPilot", "0.1.0");
        let app = make_app_with_target(
            id,
            Some("C:\\App\\script.bat".to_string()),
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

    #[test]
    fn test_launch_valid_application_executes_natively() {
        let sys_exe = get_test_system_executable();
        let id = "app_aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
        let state = AppState::new("DeskPilot", "0.1.0");

        let app = make_app_with_target(
            id,
            Some(sys_exe.to_string_lossy().to_string()),
            AvailabilityState::Available,
        );
        state.update_catalog(vec![app]).unwrap();

        let result = ApplicationLauncherService::launch(&state, id)
            .expect("Native launch of system executable must succeed");

        assert_eq!(result.application_id, id);
        assert!(
            result.process_id > 0,
            "Spawned child must receive a non-zero OS PID"
        );
    }

    #[test]
    fn test_launch_rejects_malformed_id() {
        let state = AppState::new("DeskPilot", "0.1.0");
        let err = ApplicationLauncherService::launch(&state, "malformed-id").unwrap_err();
        match err {
            DomainError::InvalidInput(msg) => {
                assert!(msg.contains("Invalid application identifier format"));
            }
            _ => panic!("Expected InvalidInput error"),
        }
    }

    #[test]
    fn test_launch_rejects_unknown_id() {
        let state = AppState::new("DeskPilot", "0.1.0");
        let id = "app_00000000000000000000000000000000";
        let err = ApplicationLauncherService::launch(&state, id).unwrap_err();
        match err {
            DomainError::NotFound(msg) => {
                assert!(msg.contains("Application not found in current catalog"));
            }
            _ => panic!("Expected NotFound error"),
        }
    }

    #[test]
    fn test_launch_rejects_unavailable_state() {
        let sys_exe = get_test_system_executable();
        let id = "app_bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
        let state = AppState::new("DeskPilot", "0.1.0");
        let app = make_app_with_target(
            id,
            Some(sys_exe.to_string_lossy().to_string()),
            AvailabilityState::IncompleteMetadata,
        );
        state.update_catalog(vec![app]).unwrap();

        let err = ApplicationLauncherService::launch(&state, id).unwrap_err();
        match err {
            DomainError::Unavailable(msg) => {
                assert!(msg.contains("not in an available state"));
            }
            _ => panic!("Expected Unavailable error"),
        }
    }

    #[test]
    fn test_launch_rejects_unc_target() {
        let state = AppState::new("DeskPilot", "0.1.0");
        let id = "app_cccccccccccccccccccccccccccccccc";
        let app = make_app_with_target(
            id,
            Some(r"\\192.168.1.1\share\app.exe".to_string()),
            AvailabilityState::Available,
        );
        state.update_catalog(vec![app]).unwrap();

        let err = ApplicationLauncherService::launch(&state, id).unwrap_err();
        match err {
            DomainError::PermissionDenied(msg) => {
                assert!(msg.contains("network shares is prohibited"));
            }
            _ => panic!("Expected PermissionDenied error for UNC target"),
        }
    }

    #[test]
    fn test_launch_rejects_device_target() {
        let state = AppState::new("DeskPilot", "0.1.0");
        let id = "app_dddddddddddddddddddddddddddddddd";
        let app = make_app_with_target(
            id,
            Some(r"\\.\COM1".to_string()),
            AvailabilityState::Available,
        );
        state.update_catalog(vec![app]).unwrap();

        let err = ApplicationLauncherService::launch(&state, id).unwrap_err();
        match err {
            DomainError::PermissionDenied(msg) => {
                assert!(msg.contains("device namespaces is prohibited"));
            }
            _ => panic!("Expected PermissionDenied error for device namespace"),
        }
    }

    #[test]
    fn test_launch_rejects_verbatim_unc_target() {
        let state = AppState::new("DeskPilot", "0.1.0");
        let id = "app_eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
        let app = make_app_with_target(
            id,
            Some(r"\\?\UNC\server\share\app.exe".to_string()),
            AvailabilityState::Available,
        );
        state.update_catalog(vec![app]).unwrap();

        let err = ApplicationLauncherService::launch(&state, id).unwrap_err();
        match err {
            DomainError::PermissionDenied(msg) => {
                assert!(msg.contains("network shares is prohibited"));
            }
            _ => panic!("Expected PermissionDenied error for verbatim UNC target"),
        }
    }

    #[test]
    fn test_launch_rejects_transient_temp_directory() {
        let temp_exe = create_temp_test_file("transient_app.exe");
        let id = "app_ffffffffffffffffffffffffffffffff";
        let state = AppState::new("DeskPilot", "0.1.0");
        let app = make_app_with_target(
            id,
            Some(temp_exe.to_string_lossy().to_string()),
            AvailabilityState::Available,
        );
        state.update_catalog(vec![app]).unwrap();

        let err = ApplicationLauncherService::launch(&state, id).unwrap_err();
        match err {
            DomainError::PermissionDenied(msg) => {
                assert!(msg.contains("temporary or transient directories is prohibited"));
                assert!(!msg.contains("AppData"));
                assert!(!msg.contains("Temp"));
                assert!(!msg.contains("transient_app.exe"));
            }
            _ => panic!("Expected PermissionDenied for transient temp file"),
        }

        let _ = std::fs::remove_file(temp_exe);
    }

    #[test]
    fn test_launch_rejects_transient_downloads_directory() {
        let id = "app_01010101010101010101010101010101";
        let state = AppState::new("DeskPilot", "0.1.0");
        let downloads_path = "C:\\Users\\Default\\Downloads\\app.exe";
        let app = make_app_with_target(
            id,
            Some(downloads_path.to_string()),
            AvailabilityState::Available,
        );
        state.update_catalog(vec![app]).unwrap();

        // Non-existent in Downloads fails missing file, but validate_execution_policy directly tests downloads:
        let policy_err =
            ApplicationLauncherService::validate_execution_policy(Path::new(downloads_path))
                .unwrap_err();
        match policy_err {
            DomainError::PermissionDenied(msg) => {
                assert!(msg.contains("temporary or transient directories is prohibited"));
                assert!(!msg.contains("Downloads"));
            }
            _ => panic!("Expected PermissionDenied for Downloads directory"),
        }
    }

    #[test]
    fn test_launch_rejects_canonical_target_mismatch() {
        let sys_exe = get_test_system_executable();
        let other_exe = PathBuf::from(r"C:\Windows\notepad.exe");
        let mismatch_res = ApplicationLauncherService::validate_target_consistency(
            &sys_exe.to_string_lossy(),
            &other_exe,
        );
        assert!(mismatch_res.is_err());
        match mismatch_res.unwrap_err() {
            DomainError::PermissionDenied(msg) => {
                assert!(msg.contains("not correspond to the registered catalog target"));
            }
            _ => panic!("Expected PermissionDenied for target mismatch"),
        }
    }

    #[test]
    fn test_execute_target_rejects_transient_policy() {
        let temp_exe = create_temp_test_file("transient_target.exe");
        let target = ValidatedLaunchTarget {
            application_id: "app_12121212121212121212121212121212".to_string(),
            display_name: "Transient Test".to_string(),
            executable_path: temp_exe.clone(),
            working_directory: temp_exe.parent().map(|p| p.to_path_buf()),
        };

        let err = ApplicationLauncherService::execute_target(&target).unwrap_err();
        match err {
            DomainError::PermissionDenied(msg) => {
                assert!(msg.contains("temporary or transient directories is prohibited"));
                assert!(!msg.contains("Temp"));
            }
            _ => panic!("Expected PermissionDenied for execute_target in transient dir"),
        }

        let _ = std::fs::remove_file(temp_exe);
    }

    #[test]
    fn test_execute_target_rejects_missing_file() {
        let target = ValidatedLaunchTarget {
            application_id: "app_13131313131313131313131313131313".to_string(),
            display_name: "Missing Target".to_string(),
            executable_path: PathBuf::from(r"C:\Program Files\DeskPilot_Missing_12345\missing.exe"),
            working_directory: Some(PathBuf::from(r"C:\Program Files\DeskPilot_Missing_12345")),
        };

        let err = ApplicationLauncherService::execute_target(&target).unwrap_err();
        match err {
            DomainError::Unavailable(msg) => {
                assert!(msg.contains("does not exist on disk"));
                assert!(!msg.contains("DeskPilot_Missing"));
            }
            _ => panic!("Expected Unavailable for missing target file"),
        }
    }

    #[test]
    fn test_spawn_error_privacy() {
        let not_found = ApplicationLauncherService::sanitize_spawn_error(std::io::Error::from(
            std::io::ErrorKind::NotFound,
        ));
        let msg1 = format!("{}", not_found);
        assert!(!msg1.contains("C:\\"));
        assert!(!msg1.contains("Users"));
        assert!(msg1.contains("does not exist on disk"));

        let perm_denied = ApplicationLauncherService::sanitize_spawn_error(std::io::Error::from(
            std::io::ErrorKind::PermissionDenied,
        ));
        let msg2 = format!("{}", perm_denied);
        assert!(!msg2.contains("C:\\"));
        assert!(msg2.contains("denied by the operating system"));

        let other = ApplicationLauncherService::sanitize_spawn_error(std::io::Error::other(
            "arbitrary internal OS error",
        ));
        let msg3 = format!("{}", other);
        assert!(!msg3.contains("arbitrary"));
        assert!(msg3.contains("Failed to spawn process"));

        let elevation_err = ApplicationLauncherService::sanitize_spawn_error(
            std::io::Error::from_raw_os_error(740),
        );
        let msg4 = format!("{}", elevation_err);
        assert!(msg4.contains("requires administrator privileges"));
        assert!(msg4.contains("standard user privileges"));
        assert!(!msg4.contains("C:\\"));
        assert!(!msg4.contains("Users"));
    }

    #[test]
    fn test_execute_target_no_caller_arguments_or_env_override() {
        let sys_exe = get_test_system_executable();
        let target = ValidatedLaunchTarget {
            application_id: "app_14141414141414141414141414141414".to_string(),
            display_name: "Direct Spawn".to_string(),
            executable_path: sys_exe.clone(),
            working_directory: sys_exe.parent().map(|p| p.to_path_buf()),
        };

        // Notice: execute_target takes ONLY &ValidatedLaunchTarget.
        // It provides no argument list, shell string, or environment override parameters.
        let result = ApplicationLauncherService::execute_target(&target)
            .expect("Direct spawn of system executable must succeed");

        assert_eq!(result.application_id, target.application_id);
        assert!(result.process_id > 0);
    }
}
