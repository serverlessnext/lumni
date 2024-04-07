mod app_loader;
mod configuration;
mod form_submit;

pub use app_loader::AppLoader;
pub use configuration::AppConfigView;

use lumni::api::handler::AppHandler;
// auto-generated via build.rs:
// - fn get_app_handler()
// - fn get_available_apps()
include!(concat!(env!("OUT_DIR"), "/generated_modules.rs"));
