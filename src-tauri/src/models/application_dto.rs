use super::application::{AvailabilityState, DiscoveredApplication};
use serde::{Deserialize, Serialize};

/// Frontend representation of application availability status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AvailabilityStateDto {
    Available,
    MissingTarget,
    IncompleteMetadata,
    Unverified,
}

impl From<AvailabilityState> for AvailabilityStateDto {
    fn from(state: AvailabilityState) -> Self {
        match state {
            AvailabilityState::Available => AvailabilityStateDto::Available,
            AvailabilityState::MissingTarget => AvailabilityStateDto::MissingTarget,
            AvailabilityState::IncompleteMetadata => AvailabilityStateDto::IncompleteMetadata,
            AvailabilityState::Unverified => AvailabilityStateDto::Unverified,
        }
    }
}

/// Frontend Data Transfer Object representing an application in the catalog.
///
/// Notice: Sensitive machine information (such as raw executable paths, usernames,
/// or registry subkeys) is strictly omitted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[allow(non_snake_case)]
pub struct ApplicationDto {
    pub id: String,
    pub displayName: String,
    pub publisher: Option<String>,
    pub version: Option<String>,
    pub sources: Vec<String>,
    pub availability: AvailabilityStateDto,
    pub isInstalled: bool,
}

impl From<DiscoveredApplication> for ApplicationDto {
    fn from(domain: DiscoveredApplication) -> Self {
        let is_installed = domain.availability == AvailabilityState::Available;
        let sources = domain
            .sources
            .iter()
            .map(|s| s.display_label().to_string())
            .collect();

        Self {
            id: domain.id,
            displayName: domain.display_name,
            publisher: domain.publisher,
            version: domain.version,
            sources,
            availability: domain.availability.into(),
            isInstalled: is_installed,
        }
    }
}

/// Result returned to the frontend by the `discover_applications` IPC command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[allow(non_snake_case)]
pub struct DiscoveryResultDto {
    pub applications: Vec<ApplicationDto>,
    pub scannedSources: Vec<String>,
    pub partialFailures: Vec<String>,
    pub totalDiscovered: usize,
}

/// Result returned to the frontend by the `launch_application` IPC command.
///
/// Contains strictly the opaque application identifier and the operating system process ID.
/// Sensitive machine paths, arguments, working directories, and environment variables are strictly omitted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LaunchResultDto {
    pub application_id: String,
    pub process_id: u32,
}
