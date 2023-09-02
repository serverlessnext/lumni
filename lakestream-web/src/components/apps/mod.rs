mod app_config;
mod form_submit;

pub use app_config::AppConfig;
pub use form_submit::AppFormSubmit;

use crate::api::handler::AppHandler;
// auto-generated via build.rs:
// - fn get_app_handler()
// - fn get_available_apps()
include!(concat!(env!("OUT_DIR"), "/generated_modules.rs"));
