pub mod error;
pub mod handler;
pub mod invoke;
pub mod types;

use crate::api::handler::AppHandler;
// auto-generated via build.rs:
// - fn get_app_handler()
// - fn get_available_apps()
include!(concat!(env!("OUT_DIR"), "/generated_modules.rs"));

pub fn find_builtin_app(app_name: &str) -> Option<Box<dyn AppHandler>> {
    let app = get_available_apps().into_iter().find(|app| {
        app.get("__uri__")
            .map_or(false, |uri| uri.starts_with("builtin::"))
            && app.get("name").map_or(false, |n| n == app_name)
    });
    match app {
        Some(app) => match app.get("__uri__") {
            Some(uri) => get_app_handler(uri),
            None => None,
        },
        None => None,
    }
}
