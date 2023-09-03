
use leptos::*;

use super::list_view::ConfigurationListView;


#[component]
pub fn AppConfiguration(cx: Scope) -> impl IntoView {
    view! { cx,
        <ConfigurationListView />
    }
}

