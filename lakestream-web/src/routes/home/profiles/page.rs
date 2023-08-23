use leptos::*;

use super::list_view::ConfigurationListView;

#[component]
pub fn UserProfiles(cx: Scope) -> impl IntoView {
    view! { cx,
        <ConfigurationListView />
    }
}
