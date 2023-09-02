use leptos::*;

pub use crate::components::apps::{get_available_apps, AppFormSubmit};
pub use crate::components::Environment;

#[component]
pub fn Console(cx: Scope) -> impl IntoView {
    // TODO: user should be able to select this
    log!("Available apps: {:?}", get_available_apps());
    let app_uri = "builtin::storage::s3::objectstore_s3".to_string();

    view! {
        cx,
        <Environment />
        <AppFormSubmit app_uri/>
        <br />
    }
}
