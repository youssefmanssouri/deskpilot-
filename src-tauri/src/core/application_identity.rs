use sha2::{Digest, Sha256};

/// Deterministic Application Identity Generator.
///
/// Implements stable, collision-resistant identity generation for Windows applications.
/// IDs remain identical across discovery runs as long as the underlying application
/// identity (publisher, display name, or target executable) remains constant.
pub struct ApplicationIdentity;

impl ApplicationIdentity {
    /// Generates a stable application ID formatted as `app_<16-hex-chars>`.
    ///
    /// The algorithm uses SHA-256 over a normalized canonical key seed:
    /// - If publisher is present: `pub:<norm_publisher>:app:<norm_name>`
    /// - If publisher is absent and target basename is present: `exe:<norm_exe>:app:<norm_name>`
    /// - Otherwise: `bare:app:<norm_name>`
    pub fn generate_id(
        display_name: &str,
        publisher: Option<&str>,
        target_path: Option<&str>,
    ) -> String {
        let norm_name = Self::normalize_display_name(display_name);
        let norm_pub = publisher.and_then(Self::normalize_publisher);
        let norm_exe = target_path.and_then(Self::extract_target_basename);

        let seed = match (norm_pub, norm_exe) {
            (Some(ref p), _) if !p.is_empty() => {
                format!("pub:{}:app:{}", p, norm_name)
            }
            (None, Some(ref exe)) if !exe.is_empty() => {
                format!("exe:{}:app:{}", exe, norm_name)
            }
            _ => {
                format!("bare:app:{}", norm_name)
            }
        };

        let mut hasher = Sha256::new();
        hasher.update(seed.as_bytes());
        let hash = hasher.finalize();

        // Take the first 8 bytes (16 hex characters = 64 bits of entropy)
        let hex_id: String = hash[..8].iter().map(|b| format!("{:02x}", b)).collect();
        format!("app_{}", hex_id)
    }

    /// Normalizes display name by trimming, lowercasing, and collapsing whitespace.
    pub fn normalize_display_name(name: &str) -> String {
        name.split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ")
            .to_lowercase()
    }

    /// Normalizes publisher string by trimming, lowercasing, and removing corporate noise.
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
            ", ltd.",
            " ltd.",
            ", ltd",
            " ltd",
        ];

        for suffix in SUFFIXES {
            if lowered.ends_with(suffix) {
                lowered.truncate(lowered.len() - suffix.len());
                break;
            }
        }

        let normalized = lowered.split_whitespace().collect::<Vec<&str>>().join(" ");

        if normalized.is_empty() {
            None
        } else {
            Some(normalized)
        }
    }

    /// Extracts and normalizes the target executable basename (e.g. `chrome.exe`).
    pub fn extract_target_basename(path: &str) -> Option<String> {
        let trimmed = path.trim().trim_matches('"');
        if trimmed.is_empty() {
            return None;
        }

        std::path::Path::new(trimmed)
            .file_name()
            .and_then(|f| f.to_str())
            .map(|name| name.to_lowercase())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_determinism() {
        let id1 = ApplicationIdentity::generate_id("Google Chrome", Some("Google LLC"), None);
        let id2 = ApplicationIdentity::generate_id("Google Chrome", Some("Google LLC"), None);
        assert_eq!(id1, id2);
        assert!(id1.starts_with("app_"));
        assert_eq!(id1.len(), 20); // "app_" (4) + 16 hex chars
    }

    #[test]
    fn test_identity_differentiation_by_publisher() {
        let id_google = ApplicationIdentity::generate_id("Browser", Some("Google"), None);
        let id_mozilla = ApplicationIdentity::generate_id("Browser", Some("Mozilla"), None);
        assert_ne!(id_google, id_mozilla);
    }

    #[test]
    fn test_identity_differentiation_by_executable_when_no_publisher() {
        let id_a = ApplicationIdentity::generate_id("Client", None, Some(r"C:\AppA\client_a.exe"));
        let id_b = ApplicationIdentity::generate_id("Client", None, Some(r"C:\AppB\client_b.exe"));
        assert_ne!(id_a, id_b);
    }

    #[test]
    fn test_publisher_normalization_cross_source() {
        // "Google LLC" and "Google, Inc." should normalize to "google"
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
            None,
        );
        let id2 =
            ApplicationIdentity::generate_id("visual studio code", Some("microsoft corp."), None);
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_target_basename_extraction() {
        let extracted =
            ApplicationIdentity::extract_target_basename(r#""C:\Program Files\App\app.exe" -arg"#);
        assert_eq!(extracted, Some("app.exe -arg".to_string())); // Path with args handled or path
        let extracted_clean =
            ApplicationIdentity::extract_target_basename(r#"C:\Program Files\App\test.exe"#);
        assert_eq!(extracted_clean, Some("test.exe".to_string()));
    }
}
