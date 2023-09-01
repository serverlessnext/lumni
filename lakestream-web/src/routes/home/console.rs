use leptos::*;

pub use crate::components::apps::AppFormSubmit;
pub use crate::components::Environment;

#[component]
pub fn Console(cx: Scope) -> impl IntoView {
    // TODO: user should be able to select this
    let app_name = "builtin::storage::s3::objectstore_s3".to_string();

    view! {
        cx,
        <Environment />
        <AppFormSubmit app_name/>
        <br />
    }
}
