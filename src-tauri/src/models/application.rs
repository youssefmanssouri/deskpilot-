use serde::{Deserialize, Serialize};

/// Discovered source provenance indicating where the application was found.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DiscoverySource {
    RegistryHklm64,
    RegistryHklm32,
    RegistryHkcu,
    StartMenuUser,
    StartMenuCommon,
    TestFixture,
}

impl DiscoverySource {
    pub fn display_label(&self) -> &'static str {
        match self {
            DiscoverySource::RegistryHklm64 => "Registry (HKLM 64-bit)",
            DiscoverySource::RegistryHklm32 => "Registry (HKLM 32-bit)",
            DiscoverySource::RegistryHkcu => "Registry (HKCU User)",
            DiscoverySource::StartMenuUser => "Start Menu (User)",
            DiscoverySource::StartMenuCommon => "Start Menu (Common)",
            DiscoverySource::TestFixture => "Test Fixture",
        }
    }
}

/// Operational availability state of a discovered application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AvailabilityState {
    /// The target executable has been verified to exist on disk.
    Available,
    /// A target path or installation folder was specified, but is missing from disk.
    MissingTarget,
    /// The application was recorded without target executable or folder metadata.
    IncompleteMetadata,
    /// A shortcut or record exists, but the target has not yet been resolved or verified.
    Unverified,
}

impl AvailabilityState {
    pub fn as_str(&self) -> &'static str {
        match self {
            AvailabilityState::Available => "Available",
            AvailabilityState::MissingTarget => "MissingTarget",
            AvailabilityState::IncompleteMetadata => "IncompleteMetadata",
            AvailabilityState::Unverified => "Unverified",
        }
    }
}

/// Internal domain model representing an identified, normalized Windows application.
///
/// Note: Sensitive operating system details (such as raw executable paths or registry keys)
/// are preserved internally for future launcher functionality, but are NEVER exposed
/// across the IPC boundary to the frontend.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiscoveredApplication {
    pub id: String,
    pub display_name: String,
    pub publisher: Option<String>,
    pub version: Option<String>,
    pub sources: Vec<DiscoverySource>,
    pub install_location: Option<String>,
    pub target_path: Option<String>,
    pub availability: AvailabilityState,
    pub discovered_at: u64,
}
