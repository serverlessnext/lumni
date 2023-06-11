
use leptos::*;

use super::list_view::ObjectStoreListView;

#[component]
pub fn ObjectStores(cx: Scope) -> impl IntoView {
    view! { cx,
        <ObjectStoreListView />
    }
}

