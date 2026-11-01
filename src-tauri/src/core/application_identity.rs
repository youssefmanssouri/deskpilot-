use sha2::{Digest, Sha256};
use std::path::Path;

/// Deterministic Application Identity Generator.
///
/// Implements stable, collision-resistant identity generation for Windows applications.
/// IDs remain identical across discovery runs as long as the underlying application
/// identity (publisher, display name, target executable) remains constant.
pub struct ApplicationIdentity;

impl ApplicationIdentity {
    /// Generates a stable, 128-bit collision-resistant application ID formatted as `app_<32-hex-chars>`.
    ///
    /// The algorithm uses SHA-256 over a normalized canonical key seed:
    /// - If publisher and target are present: `pub:<norm_publisher>:exe:<norm_exe>:app:<norm_name>`
    /// - If publisher is present, target is absent: `pub:<norm_publisher>:app:<norm_name>`
    /// - If publisher is absent, target is present: `exe:<norm_exe>:app:<norm_name>`
    /// - Otherwise: `bare:app:<norm_name>`
    pub fn generate_id(
        display_name: &str,
        publisher: Option<&str>,
        target_path: Option<&str>,
    ) -> String {
        let norm_name = Self::normalize_display_name(display_name);
        let norm_pub = publisher.and_then(Self::normalize_publisher);
        let norm_exe = target_path.and_then(Self::extract_target_basename);

        let seed = match (&norm_pub, &norm_exe) {
            (Some(p), Some(exe)) if !p.is_empty() && !exe.is_empty() => {
                format!("pub:{}:exe:{}:app:{}", p, exe, norm_name)
            }
            (Some(p), _) if !p.is_empty() => {
                format!("pub:{}:app:{}", p, norm_name)
            }
            (None, Some(exe)) if !exe.is_empty() => {
                format!("exe:{}:app:{}", exe, norm_name)
            }
            _ => {
                format!("bare:app:{}", norm_name)
            }
        };

        let mut hasher = Sha256::new();
        hasher.update(seed.as_bytes());
        let hash = hasher.finalize();

        // Take the first 16 bytes (32 hex characters = 128 bits of cryptographic entropy)
        let hex_id: String = hash[..16].iter().map(|b| format!("{:02x}", b)).collect();
        format!("app_{}", hex_id)
    }

    /// Normalizes display name by trimming, lowercasing, and collapsing whitespace.
    pub fn normalize_display_name(name: &str) -> String {
        name.split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ")
            .to_lowercase()
    }

    /// Normalizes publisher string by trimming, lowercasing, and stripping corporate legal suffixes.
    pub fn normalize_publisher(pub_name: &str) -> Option<String> {
        let trimmed = pub_name.trim();
        if trimmed.is_empty() {
            return None;
        }

        let mut lowered = trimmed.to_lowercase();

        // Strip common corporate legal suffixes to ensure cross-source parity
        const SUFFIXES: &[&str] = &[
            ", inc.",
            " inc.",
            ", inc",
            " inc",
            ", llc.",
            " llc.",
            ", llc",
            " llc",
            ", corp.",
            " corp.",
            ", corp",
            " corp",
            ", corporation",
            " corporation",
            ", gmbh",
            " gmbh",
            ", pte. ltd.",
            " pte. ltd.",
            ", co., ltd.",
            " co., ltd.",
            ", co., ltd",
            " co., ltd",
            ", ltd.",
            " ltd.",
            ", ltd",
            " ltd",
            ", s.a.",
            " s.a.",
            ", s.r.o.",
            " s.r.o.",
            ", sarl",
            " sarl",
            ", pty ltd",
            " pty ltd",
            ", plc",
            " plc",
        ];

        let mut matched = true;
        while matched {
            matched = false;
            let current = lowered.trim().trim_end_matches([',', '.', ';']).trim();
            for suffix in SUFFIXES {
                if current.ends_with(suffix) {
                    lowered = current[..current.len() - suffix.len()].to_string();
                    matched = true;
                    break;
                }
            }
        }

        let normalized = lowered
            .trim()
            .trim_end_matches([',', '.', ';'])
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ");

        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    }

    /// Extracts and normalizes the target executable basename or directory stem.
    ///
    /// Correctly parses quoted paths, command-line arguments, switches, and trailing slashes.
    pub fn extract_target_basename(raw_path: &str) -> Option<String> {
        let trimmed = raw_path.trim();
        if trimmed.is_empty() {
            return None;
        }

        // 1. Isolate the executable/file path from command line arguments
        let clean_path = if trimmed.starts_with('"') {
            // Find the closing quote
            if let Some(end_idx) = trimmed[1..].find('"') {
                &trimmed[1..=end_idx]
            } else {
                trimmed.trim_matches('"')
            }
        } else {
            // If unquoted, check if common executable extensions exist followed by a delimiter
            let lowered = trimmed.to_lowercase();
            let mut cut_idx = None;

            for ext in &[".exe", ".cmd", ".bat", ".msi"] {
                if let Some(pos) = lowered.find(ext) {
                    let end_pos = pos + ext.len();
                    // Must be at the end of string or followed by whitespace/delimiter
                    if end_pos == trimmed.len()
                        || trimmed.as_bytes()[end_pos].is_ascii_whitespace()
                        || trimmed.as_bytes()[end_pos] == b','
                    {
                        cut_idx = Some(end_pos);
                        break;
                    }
                }
            }

            if let Some(idx) = cut_idx {
                &trimmed[..idx]
            } else {
                // If no executable extension found, check if it's a directory path or single token
                trimmed.trim_end_matches(['\\', '/'])
            }
        };

        let clean_path = clean_path.trim().trim_end_matches(['\\', '/']);
        if clean_path.is_empty() {
            return None;
        }

        // 2. Extract basename from path
        Path::new(clean_path)
            .file_name()
            .and_then(|f| f.to_str())
            .map(|name| name.trim().to_lowercase())
            .filter(|name| !name.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_determinism_and_entropy() {
        let id1 = ApplicationIdentity::generate_id(
            "Google Chrome",
            Some("Google LLC"),
            Some(r"C:\Program Files\Google\Chrome\Application\chrome.exe"),
        );
        let id2 = ApplicationIdentity::generate_id(
            "Google Chrome",
            Some("Google LLC"),
            Some(r"C:\Program Files\Google\Chrome\Application\chrome.exe"),
        );
        assert_eq!(id1, id2);
        assert!(id1.starts_with("app_"));
        // "app_" (4) + 32 hex chars = 36 chars (128 bits)
        assert_eq!(id1.len(), 36);
        assert!(
            id1[4..].chars().all(|c| c.is_ascii_hexdigit()),
            "ID must be valid hexadecimal"
        );
    }

    #[test]
    fn test_identity_differentiation_by_publisher() {
        let id_google = ApplicationIdentity::generate_id("Browser", Some("Google"), None);
        let id_mozilla = ApplicationIdentity::generate_id("Browser", Some("Mozilla"), None);
        assert_ne!(id_google, id_mozilla);
    }

    #[test]
    fn test_identity_differentiation_same_publisher_different_executables() {
        // Multiple distinct tools from the same publisher must not collide
        let id_teams = ApplicationIdentity::generate_id(
            "Client",
            Some("Microsoft Corporation"),
            Some(r"C:\Program Files\Microsoft\Teams\teams.exe"),
        );
        let id_office = ApplicationIdentity::generate_id(
            "Client",
            Some("Microsoft Corporation"),
            Some(r"C:\Program Files\Microsoft\Office\winword.exe"),
        );
        assert_ne!(
            id_teams, id_office,
            "Distinct executables from the same publisher must have distinct IDs"
        );
    }

    #[test]
    fn test_identity_differentiation_multiple_editions_or_installs() {
        let id_py311 = ApplicationIdentity::generate_id(
            "Python",
            Some("Python Software Foundation"),
            Some(r"C:\Users\user\AppData\Local\Programs\Python311\"),
        );
        let id_py312 = ApplicationIdentity::generate_id(
            "Python",
            Some("Python Software Foundation"),
            Some(r"C:\Users\user\AppData\Local\Programs\Python312\"),
        );
        assert_ne!(
            id_py311, id_py312,
            "Different edition install folders must produce distinct IDs"
        );
    }

    #[test]
    fn test_publisher_normalization_cross_source() {
        let id_llc = ApplicationIdentity::generate_id("Chrome", Some("Google LLC"), None);
        let id_inc = ApplicationIdentity::generate_id("Chrome", Some("Google, Inc."), None);
        let id_bare = ApplicationIdentity::generate_id("Chrome", Some("Google"), None);
        assert_eq!(id_llc, id_inc);
        assert_eq!(id_llc, id_bare);
    }

    #[test]
    fn test_whitespace_and_case_insensitivity() {
        let id1 = ApplicationIdentity::generate_id(
            "  Visual Studio   Code  ",
            Some("Microsoft Corporation"),
            Some(r#""C:\Program Files\Microsoft VS Code\Code.exe" --profile default"#),
        );
        let id2 = ApplicationIdentity::generate_id(
            "visual studio code",
            Some("microsoft corp."),
            Some(r#"C:\Program Files\Microsoft VS Code\code.exe"#),
        );
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_target_basename_extraction_robustness() {
        // Quoted with trailing arguments
        let extracted =
            ApplicationIdentity::extract_target_basename(r#""C:\Program Files\App\app.exe" -arg"#);
        assert_eq!(extracted, Some("app.exe".to_string()));

        // Quoted with spaces and no arguments
        let extracted_clean =
            ApplicationIdentity::extract_target_basename(r#""C:\Program Files\App\test.exe""#);
        assert_eq!(extracted_clean, Some("test.exe".to_string()));

        // Unquoted with switches
        let extracted_unquoted_switches = ApplicationIdentity::extract_target_basename(
            r#"C:\Tools\client.exe /silent /norestart"#,
        );
        assert_eq!(extracted_unquoted_switches, Some("client.exe".to_string()));

        // Directory with trailing slash
        let extracted_dir =
            ApplicationIdentity::extract_target_basename(r#"C:\Program Files\Python312\"#);
        assert_eq!(extracted_dir, Some("python312".to_string()));

        // Empty or whitespace
        assert_eq!(ApplicationIdentity::extract_target_basename(""), None);
        assert_eq!(ApplicationIdentity::extract_target_basename("   "), None);
    }
}
