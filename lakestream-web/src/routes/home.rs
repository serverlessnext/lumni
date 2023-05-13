use leptos::*;

use std::sync::Arc;

use crate::base::connector::{LakestreamHandler, get_config};


#[component]
pub fn Home(cx: Scope) -> impl IntoView {

    let config = get_config();
    let handler = Arc::new(LakestreamHandler::new(config));

    let (count, set_count) = create_signal(cx, 0);

    let async_data = {
        let handler_clone = Arc::clone(&handler);
        create_resource(cx, count, move |count| {
            let handler_clone = Arc::clone(&handler_clone);
            async move { handler_clone.list_objects_demo(count).await }
        })
    };

    let async_result = move || {
        log!("async_result");
        async_data
            .read(cx)
            .map(|files| format!("Files: {:?}", files))
            .unwrap_or_else(|| "Loading...".into())
    };


    let loading = async_data.loading();
    let is_loading = move || if loading() { "Loading..." } else { "Idle." };


    view! { cx,
        <button
            class="bg-amber-600 hover:bg-sky-700 px-5 py-3 text-white rounded-lg"
            on:click=move |_| {
                set_count.update(|n| *n += 1);
            }
        >
            "Get data"
        </button>
        <p class="px-10 pb-10 text-left">
            {is_loading}
            <br/>
            {async_result}
        </p>
    }
}
