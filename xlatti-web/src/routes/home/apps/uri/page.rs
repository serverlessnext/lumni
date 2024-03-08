use leptos::*;

use crate::components::apps::AppLoader;

#[component]
pub fn AppUri() -> impl IntoView {
    // TODO: make app selectable
    let app_uri = "builtin::extract::objectstore".to_string();

    view! {
        <AppLoader app_uri/>
    }
}
