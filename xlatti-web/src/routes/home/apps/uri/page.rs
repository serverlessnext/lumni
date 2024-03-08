use leptos::*;

use crate::components::apps::AppLoader;

#[component]
pub fn AppUri(cx: Scope) -> impl IntoView {
    // TODO: make app selectable
    let app_uri = "builtin::extract::objectstore".to_string();

    view! {
        cx,
        <AppLoader app_uri/>
    }
}
