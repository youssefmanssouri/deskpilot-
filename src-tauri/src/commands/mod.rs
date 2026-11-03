pub mod applications;
pub mod processes;
pub mod status;

pub use applications::{discover_applications, launch_application};
pub use processes::list_processes;
pub use status::{get_app_status, validate_command_intent};
