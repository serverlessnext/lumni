use leptos::{component, tracing, view, IntoView, Scope};

use crate::components::object_store_list::ObjectStoreListView;

#[component]
pub fn ObjectStores(cx: Scope) -> impl IntoView {
    view! { cx,
        <ObjectStoreListView />
    }
}
