use serde::{Deserialize, Serialize};
use std::fmt;

/// Application-level domain error categories.
///
/// Internal errors, filesystem errors, and system details must be mapped into
/// one of these categories before returning across the IPC boundary to the frontend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    /// The input provided by the frontend failed validation (e.g. malformed ID, invalid format).
    InvalidInput(String),
    /// The requested resource (application, operation, note, etc.) was not found.
    NotFound(String),
    /// The operation is disallowed by security boundaries or capability policies.
    PermissionDenied(String),
    /// A required subsystem or service is temporarily unavailable.
    Unavailable(String),
    /// An operation attempted by the core failed execution safely.
    OperationFailed(String),
    /// An unexpected internal system failure occurred.
    InternalError(String),
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DomainError::InvalidInput(msg) => write!(f, "Invalid input: {}", msg),
            DomainError::NotFound(msg) => write!(f, "Not found: {}", msg),
            DomainError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            DomainError::Unavailable(msg) => write!(f, "Service unavailable: {}", msg),
            DomainError::OperationFailed(msg) => write!(f, "Operation failed: {}", msg),
            DomainError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for DomainError {}

/// Safe Data Transfer Object for returning errors over Tauri IPC.
///
/// This structure guarantees that internal details (paths, stack traces, environment)
/// are never leaked across the IPC boundary to the untrusted frontend.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IpcErrorDto {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

impl From<DomainError> for IpcErrorDto {
    fn from(error: DomainError) -> Self {
        match error {
            DomainError::InvalidInput(msg) => IpcErrorDto {
                code: "INVALID_INPUT".to_string(),
                message: msg,
                details: None,
            },
            DomainError::NotFound(msg) => IpcErrorDto {
                code: "NOT_FOUND".to_string(),
                message: msg,
                details: None,
            },
            DomainError::PermissionDenied(msg) => IpcErrorDto {
                code: "PERMISSION_DENIED".to_string(),
                message: msg,
                details: None,
            },
            DomainError::Unavailable(msg) => IpcErrorDto {
                code: "UNAVAILABLE".to_string(),
                message: msg,
                details: None,
            },
            DomainError::OperationFailed(msg) => IpcErrorDto {
                code: "OPERATION_FAILED".to_string(),
                message: msg,
                details: None,
            },
            // For internal errors, the client receives a sanitized, non-leaking message.
            DomainError::InternalError(_internal_details) => IpcErrorDto {
                code: "INTERNAL_ERROR".to_string(),
                message: "An internal system error occurred. Please consult backend logs."
                    .to_string(),
                details: None,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_mapping_categories() {
        let err = DomainError::InvalidInput("Malformed ID".to_string());
        let dto: IpcErrorDto = err.into();
        assert_eq!(dto.code, "INVALID_INPUT");
        assert_eq!(dto.message, "Malformed ID");
        assert!(dto.details.is_none());

        let err = DomainError::NotFound("App not found".to_string());
        let dto: IpcErrorDto = err.into();
        assert_eq!(dto.code, "NOT_FOUND");
        assert_eq!(dto.message, "App not found");

        let err = DomainError::PermissionDenied("Access to path blocked".to_string());
        let dto: IpcErrorDto = err.into();
        assert_eq!(dto.code, "PERMISSION_DENIED");
        assert_eq!(dto.message, "Access to path blocked");
    }

    #[test]
    fn test_internal_error_does_not_leak_details() {
        let sensitive_info =
            "Database panic at C:\\secret\\path\\db.sqlite: permission denied".to_string();
        let err = DomainError::InternalError(sensitive_info.clone());
        let dto: IpcErrorDto = err.into();

        assert_eq!(dto.code, "INTERNAL_ERROR");
        assert!(!dto.message.contains("secret"));
        assert!(!dto.message.contains("C:\\"));
        assert_eq!(
            dto.message,
            "An internal system error occurred. Please consult backend logs."
        );
        assert!(dto.details.is_none());
    }

    #[test]
    fn test_serialization_json() {
        let dto = IpcErrorDto {
            code: "INVALID_INPUT".to_string(),
            message: "Parameter missing".to_string(),
            details: None,
        };
        let serialized = serde_json::to_string(&dto).expect("Serialization failed");
        assert_eq!(
            serialized,
            r#"{"code":"INVALID_INPUT","message":"Parameter missing"}"#
        );
    }
}
