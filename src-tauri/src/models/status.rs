use serde::{Deserialize, Serialize};

/// Internal domain model representing the current health and status of the application.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppStatus {
    pub app_name: String,
    pub version: String,
    pub status: String,
    pub uptime_seconds: u64,
    pub environment: String,
}

impl AppStatus {
    pub fn new(
        app_name: String,
        version: String,
        status: String,
        uptime_seconds: u64,
        environment: String,
    ) -> Self {
        Self {
            app_name,
            version,
            status,
            uptime_seconds,
            environment,
        }
    }
}
