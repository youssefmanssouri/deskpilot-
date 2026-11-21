pub mod applications;
pub mod history;
pub mod icons;
pub mod launcher;
pub mod processes;
pub mod settings;
pub mod status;
pub mod system;

pub use applications::{discover_applications, launch_application};
pub use history::{clear_launch_history, get_launch_history};
pub use launcher::{get_launcher_shortcut, hide_launcher, list_applications, show_main_window};
pub use processes::list_processes;
pub use status::{get_app_status, is_running_elevated, validate_command_intent};
pub use system::get_system_metrics;
