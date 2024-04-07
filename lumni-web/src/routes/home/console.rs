use leptos::logging::log;
use leptos::*;

pub use lumni::api::get_available_apps;
pub use crate::components::apps::AppLoader;
pub use crate::components::Environment;

#[component]
pub fn Console() -> impl IntoView {
    // TODO: user should be able to select this
    log!("Available apps: {:?}", get_available_apps());
    let app_uri = "builtin::extract::objectstore".to_string();

    view! {
        <Environment />
        <AppLoader app_uri/>
        <br />
    }
}
