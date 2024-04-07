pub mod error;
pub mod handler;
pub mod invoke;
pub mod types;

use crate::api::handler::AppHandler;
// auto-generated via build.rs:
// - fn get_app_handler()
// - fn get_available_apps()
include!(concat!(env!("OUT_DIR"), "/generated_modules.rs"));
