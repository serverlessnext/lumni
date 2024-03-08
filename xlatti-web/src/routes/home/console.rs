use leptos::*;
use leptos::logging::log;
pub use crate::components::apps::{get_available_apps, AppLoader};
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
