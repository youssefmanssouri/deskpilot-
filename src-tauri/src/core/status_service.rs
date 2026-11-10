use crate::error::DomainError;
use crate::models::{AppStatus, CommandIntentDto};
use crate::security::InputGuard;
use crate::state::AppState;

/// Core domain service for managing and reporting application status and validating intents.
pub struct StatusService;

impl StatusService {
    /// Retrieves the current domain application status from state.
    pub fn get_status(state: &AppState) -> Result<AppStatus, DomainError> {
        let status_label = if state.is_healthy() {
            "ready".to_string()
        } else {
            "degraded".to_string()
        };

        Ok(AppStatus::new(
            state.app_name().to_string(),
            state.version().to_string(),
            status_label,
            state.uptime_seconds(),
            "desktop".to_string(),
        ))
    }

    /// Validates an incoming domain command intent according to security boundaries.
    pub fn validate_intent(intent: &CommandIntentDto) -> Result<(), DomainError> {
        // Validate intent identifier
        InputGuard::validate_safe_identifier(&intent.intent_id)?;

        // Validate action type against allowed policy
        InputGuard::validate_action_type(&intent.action_type)?;

        // Validate optional target identifier if present
        if let Some(ref target) = intent.target_id {
            InputGuard::validate_safe_identifier(target)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_status_healthy() {
        let state = AppState::new("DeskPilot", env!("CARGO_PKG_VERSION"));
        let status = StatusService::get_status(&state).expect("Failed to get status");
        assert_eq!(status.app_name, "DeskPilot");
        assert_eq!(status.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(status.version, "0.1.1");
        assert_eq!(status.status, "ready");
        assert_eq!(status.environment, "desktop");
    }

    #[test]
    fn test_get_status_reports_cargo_package_version_dto() {
        let state = AppState::new("DeskPilot", env!("CARGO_PKG_VERSION"));
        let status = StatusService::get_status(&state).expect("Failed to get status");
        let dto: crate::models::AppStatusDto = status.into();
        assert_eq!(dto.version, "0.1.1");
        assert_eq!(dto.version, env!("CARGO_PKG_VERSION"));
        assert_eq!(dto.app_name, "DeskPilot");
        assert_eq!(dto.status, "ready");
    }

    #[test]
    fn test_get_status_degraded() {
        let state = AppState::new("DeskPilot", env!("CARGO_PKG_VERSION"));
        state.set_healthy(false);
        let status = StatusService::get_status(&state).expect("Failed to get status");
        assert_eq!(status.status, "degraded");
    }

    #[test]
    fn test_validate_intent_success() {
        let intent = CommandIntentDto {
            intent_id: "intent-1".to_string(),
            action_type: "status".to_string(),
            target_id: Some("system-metrics".to_string()),
        };
        assert!(StatusService::validate_intent(&intent).is_ok());
    }

    #[test]
    fn test_validate_intent_invalid_action() {
        let intent = CommandIntentDto {
            intent_id: "intent-2".to_string(),
            action_type: "unauthorized_action".to_string(),
            target_id: None,
        };
        let err = StatusService::validate_intent(&intent).unwrap_err();
        assert!(matches!(err, DomainError::PermissionDenied(_)));
    }

    #[test]
    fn test_validate_intent_path_traversal_target() {
        let intent = CommandIntentDto {
            intent_id: "intent-3".to_string(),
            action_type: "inspect".to_string(),
            target_id: Some("../../../secret.txt".to_string()),
        };
        let err = StatusService::validate_intent(&intent).unwrap_err();
        assert!(matches!(err, DomainError::InvalidInput(_)));
    }
}
