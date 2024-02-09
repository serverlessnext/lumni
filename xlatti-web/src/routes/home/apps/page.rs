use std::collections::HashMap;

use leptos::*;

use crate::components::apps::get_available_apps;

#[component]
pub fn Apps(cx: Scope) -> impl IntoView {
    let apps = get_available_apps().into_iter().collect::<Vec<_>>();

    view! {
        cx,
        <div class="mt-0 p-4 bg-gray-200 rounded shadow w-full flex flex-wrap space-x-4">
        <ul>
            <For
                each={move || apps.clone()}
                key=|item| item.get("__uri__").unwrap().to_string()
                view=move |cx, item| view! { cx, <AppListItem app={item} /> }
            />
        </ul>
        </div>
    }
}

#[component]
fn AppListItem(cx: Scope, app: HashMap<String, String>) -> impl IntoView {
    let uri = app
        .get("__uri__")
        .cloned()
        .expect("Expected '__uri__' key in app hashmap.");
    let name = app
        .get("name")
        .cloned()
        .expect("Expected 'name' key in app hashmap.");

    view! {
        cx,
        <a href={format!("/apps/{}", uri)} class="flex min-w-[16rem] w-1/5 max-w-sm py-4 px-4 bg-green-400 text-gray-800 hover:bg-green-500 transition duration-300 ease-in-out font-mono font-bold rounded mb-2">{name}</a>
    }
}
