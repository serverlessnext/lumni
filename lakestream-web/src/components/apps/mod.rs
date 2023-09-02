mod form_submit;
mod app_config;

pub use app_config::AppConfig;
pub use form_submit::AppFormSubmit;

// generated fn get_app_handler()
include!(concat!(env!("OUT_DIR"), "/generated_modules.rs"));
