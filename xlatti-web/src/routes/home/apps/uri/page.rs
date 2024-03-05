use leptos::*;

use super::list_view::ConfigurationListView;

#[component]
pub fn AppConfiguration(cx: Scope) -> impl IntoView {
    // TODO: make app selectable
    let app_uri = "builtin::extract::objectstore".to_string();
    view! { cx,
        <ConfigurationListView app_uri/>
    }
}
