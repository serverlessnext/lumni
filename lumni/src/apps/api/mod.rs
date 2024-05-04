pub mod error;
pub mod handler;
pub mod invoke;
pub mod spec;
pub mod types;

use handler::AppHandler;
// auto-generated via build.rs:
// - fn get_app_handler()
// - fn get_available_apps()
include!(concat!(env!("OUT_DIR"), "/generated_modules.rs"));


#[macro_export]
macro_rules! impl_app_handler {
    // mandatory boilerplate for the AppHandler trait
    () => {
        fn clone_box(&self) -> Box<dyn AppHandler> {
            Box::new(self.clone())
        }

        fn load_specification(&self) -> &str {
            include_str!("../spec.yaml")
        }
    };
}

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
