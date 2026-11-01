use super::status::AppStatus;
use serde::{Deserialize, Serialize};

/// Frontend Data Transfer Object for application status.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppStatusDto {
    pub app_name: String,
    pub version: String,
    pub status: String,
    pub uptime_seconds: u64,
    pub environment: String,
}

impl From<AppStatus> for AppStatusDto {
    fn from(domain: AppStatus) -> Self {
        Self {
            app_name: domain.app_name,
            version: domain.version,
            status: domain.status,
            uptime_seconds: domain.uptime_seconds,
            environment: domain.environment,
        }
    }
}

/// Example high-level command intent sent from the frontend.
///
/// Notice: The frontend specifies an intent (`action_type` + `target_id`),
/// NEVER raw shell strings or executable paths.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommandIntentDto {
    pub intent_id: String,
    pub action_type: String,
    pub target_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dto_serialization() {
        let domain = AppStatus::new(
            "DeskPilot".to_string(),
            "0.1.0".to_string(),
            "ready".to_string(),
            42,
            "desktop".to_string(),
        );
        let dto: AppStatusDto = domain.into();

        let json = serde_json::to_string(&dto).expect("Serialization failed");
        assert!(json.contains(r#""appName":"DeskPilot""#));
        assert!(json.contains(r#""uptimeSeconds":42"#));
        assert!(json.contains(r#""status":"ready""#));
    }

    #[test]
    fn test_intent_deserialization() {
        let json = r#"{"intentId":"cmd-1","actionType":"inspect","targetId":"app-123"}"#;
        let dto: CommandIntentDto = serde_json::from_str(json).expect("Deserialization failed");
        assert_eq!(dto.intent_id, "cmd-1");
        assert_eq!(dto.action_type, "inspect");
        assert_eq!(dto.target_id, Some("app-123".to_string()));
    }
}
