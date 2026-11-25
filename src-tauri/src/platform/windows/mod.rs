pub mod application_discovery;
pub mod autostart;
pub mod elevation;
pub mod icon_extractor;
pub mod os_version;
pub mod process_provider;
pub mod system_provider;

pub use application_discovery::{RawDiscoveredEntry, WindowsApplicationScanner};
pub use autostart::{
    WindowsRunKeyStore, disable_start_with_windows, enable_start_with_windows,
    inspect_start_with_windows_state, is_start_with_windows_enabled,
};
pub use elevation::is_process_elevated;
pub use icon_extractor::extract_icon_rgba;
pub use os_version::windows_build_number;
pub use process_provider::{
    RawProcessCandidate, WindowsProcessProvider, filetime_ticks_to_iso8601,
};
pub use system_provider::WindowsSystemProvider;
