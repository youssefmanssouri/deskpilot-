use crate::error::DomainError;

/// Trust boundary validation utilities.
///
/// Ensures inputs crossing the IPC boundary from the untrusted frontend
/// strictly conform to format and security constraints.
pub struct InputGuard;

impl InputGuard {
    /// Validates that an identifier (such as an application ID or intent ID)
    /// does not contain path traversal tokens, null bytes, or dangerous control characters.
    pub fn validate_safe_identifier(id: &str) -> Result<(), DomainError> {
        if id.trim().is_empty() {
            return Err(DomainError::InvalidInput(
                "Identifier cannot be empty".to_string(),
            ));
        }

        if id.len() > 128 {
            return Err(DomainError::InvalidInput(
                "Identifier exceeds maximum length (128)".to_string(),
            ));
        }

        // Reject null bytes, path traversal sequences, and directory separators
        if id.contains('\0') || id.contains("..") || id.contains('/') || id.contains('\\') {
            return Err(DomainError::InvalidInput(
                "Identifier contains prohibited path or control characters".to_string(),
            ));
        }

        // Whitelist safe characters: alphanumeric, dashes, underscores, dots, colons
        let is_valid = id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == ':');
        if !is_valid {
            return Err(DomainError::InvalidInput(
                "Identifier contains invalid characters (allowed: alphanumeric, -, _, ., :)"
                    .to_string(),
            ));
        }

        Ok(())
    }

    /// Validates that an action type string is recognized by the security policy whitelist.
    pub fn validate_action_type(action: &str) -> Result<(), DomainError> {
        const ALLOWED_ACTIONS: &[&str] = &["inspect", "status", "query", "preview"];

        if !ALLOWED_ACTIONS.contains(&action) {
            return Err(DomainError::PermissionDenied(format!(
                "Action '{}' is not permitted by current security boundary",
                action
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_identifiers() {
        assert!(InputGuard::validate_safe_identifier("app-123").is_ok());
        assert!(InputGuard::validate_safe_identifier("sys.status:node_1").is_ok());
        assert!(
            InputGuard::validate_safe_identifier("uuid_550e8400-e29b-41d4-a716-446655440000")
                .is_ok()
        );
    }

    #[test]
    fn test_rejects_path_traversal_and_injection() {
        assert!(InputGuard::validate_safe_identifier("../etc/passwd").is_err());
        assert!(InputGuard::validate_safe_identifier("..\\windows\\system32").is_err());
        assert!(InputGuard::validate_safe_identifier("app/sub").is_err());
        assert!(InputGuard::validate_safe_identifier("app\\sub").is_err());
        assert!(InputGuard::validate_safe_identifier("id\0null").is_err());
        assert!(InputGuard::validate_safe_identifier("cmd;rm").is_err());
        assert!(InputGuard::validate_safe_identifier("test`whoami`").is_err());
        assert!(InputGuard::validate_safe_identifier("").is_err());
        assert!(InputGuard::validate_safe_identifier("   ").is_err());
    }

    #[test]
    fn test_action_whitelist() {
        assert!(InputGuard::validate_action_type("inspect").is_ok());
        assert!(InputGuard::validate_action_type("status").is_ok());
        assert!(InputGuard::validate_action_type("execute_arbitrary_shell").is_err());
        assert!(InputGuard::validate_action_type("delete_root").is_err());
    }
}
