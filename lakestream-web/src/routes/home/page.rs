use leptos::*;
use leptos_router::{use_location, Outlet};
use wasm_bindgen::JsValue;
use web_sys::window;

#[component]
pub fn Home(cx: Scope) -> impl IntoView {
    let page_path = use_location(cx).pathname;

    if page_path.get().ends_with("/") {
        // on first page load, rewrite url to /console
        // this avoids are more costly redirect
        if let Some(window) = window() {
            window
                .history()
                .unwrap()
                .push_state_with_url(&JsValue::UNDEFINED, "", Some("/console"))
                .unwrap();
        }
    }

    view! {
        cx,
        <nav class="bg-black">
            <div class="flex">
                {move ||
                    view! { cx,
                       <a href="/console"
                            class={
                                if page_path.get().ends_with("/console") || page_path.get().ends_with("/") {
                                    "bg-green-500/20 text-green-500 border-b-2 font-mono border-green-900 px-3 py-1 text-sm mr-4"
                                } else {
                                    "text-green-300 hover:bg-green-500/20 font-mono hover:text-green-100 px-3 py-1 text-sm mr-4"
                                }
                            }>"Console"</a>
                       <a href="/profiles"
                            class={
                                if page_path.get().ends_with("/profiles") {
                                    "bg-green-500/20 text-green-500 border-b-2 font-mono border-green-900 px-3 py-1 text-sm mr-4"
                                } else {
                                    "text-green-300 hover:bg-green-500/20 font-mono hover:text-green-100 px-3 py-1 text-sm mr-4"
                                }
                            }>"Profiles"</a>

                    }
                }
            </div>
        </nav>

        // <Console /> or <Environment /> will be rendered here
        // as these are sub-routes of Home
        <main>
            <Outlet />
        </main>
    }
}
