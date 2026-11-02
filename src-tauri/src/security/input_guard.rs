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

    /// Validates that an application identifier strictly conforms to the `app_[0-9a-f]{32}` format.
    pub fn validate_application_id(id: &str) -> Result<(), DomainError> {
        if id.len() != 36 || !id.starts_with("app_") {
            return Err(DomainError::InvalidInput(
                "Invalid application identifier format. Expected 'app_' followed by 32 lowercase hexadecimal characters."
                    .to_string(),
            ));
        }

        let hex_part = &id[4..];
        if !hex_part
            .chars()
            .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c))
        {
            return Err(DomainError::InvalidInput(
                "Application identifier contains invalid characters. Only lowercase hexadecimal characters are allowed."
                    .to_string(),
            ));
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

    #[test]
    fn test_validate_application_id() {
        // Valid lowercase 36-char ID
        assert!(
            InputGuard::validate_application_id("app_0123456789abcdef0123456789abcdef").is_ok()
        );
        // Uppercase hexadecimal strictly rejected
        assert!(
            InputGuard::validate_application_id("app_A1B2C3D4E5F678901234567890ABCDEF").is_err()
        );
        assert!(
            InputGuard::validate_application_id("app_0123456789abcdef0123456789abcdeF").is_err()
        );
        // Whitespace rejection (no trimming)
        assert!(
            InputGuard::validate_application_id(" app_0123456789abcdef0123456789abcdef").is_err()
        );
        assert!(
            InputGuard::validate_application_id("app_0123456789abcdef0123456789abcdef ").is_err()
        );
        assert!(
            InputGuard::validate_application_id("app_0123456789abcdef0123456789abcde\n").is_err()
        );
        // Length rejection
        assert!(InputGuard::validate_application_id("app_short").is_err());
        assert!(
            InputGuard::validate_application_id("app_0123456789abcdef0123456789abcdef00").is_err()
        );
        // Invalid characters
        assert!(
            InputGuard::validate_application_id("app_0123456789abcdef0123456789abcdeg").is_err()
        );
        assert!(
            InputGuard::validate_application_id("app_0123456789abcdef0123456789abcde-").is_err()
        );
        assert!(InputGuard::validate_application_id("0123456789abcdef0123456789abcdef").is_err());
        assert!(InputGuard::validate_application_id("").is_err());
        // Traversal / injection attempts
        assert!(
            InputGuard::validate_application_id("../app_0123456789abcdef0123456789abcdef").is_err()
        );
        assert!(
            InputGuard::validate_application_id("app_0123456789abcdef0123456789abc/ef").is_err()
        );
        assert!(
            InputGuard::validate_application_id("app_0123456789abcdef0123456789abc\\ef").is_err()
        );
    }
}
