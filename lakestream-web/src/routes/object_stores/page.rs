use leptos::{component, tracing, view, IntoView, Scope};

use crate::components::object_stores::ObjectStoreConfigurator;

#[component]
pub fn ObjectStores(cx: Scope) -> impl IntoView {
    view! { cx,
        <ObjectStoreConfigurator />
    }
}
