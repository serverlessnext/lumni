use leptos::*;
use leptos_router::{use_location, Outlet};
use wasm_bindgen::JsValue;
use web_sys::window;

#[component]
pub fn Home(cx: Scope) -> impl IntoView {
    let page_path = use_location(cx).pathname;

    if page_path.get_untracked().ends_with("/") {
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
        <nav class="bg-slate-100 mb-2">
            <div class="flex">
                {move ||
                    view! { cx,
                       <a href="/console"
                            class={
                                if page_path.get().ends_with("/console") || page_path.get().ends_with("/") {
                                    "bg-green-200 text-black border-b-2 font-medium font-mono border-black px-3 py-1 text-sm mr-4"
                                } else {
                                    "text-black hover:bg-green-200 font-mono hover:text-black px-3 py-1 text-sm mr-4"
                                }
                            }>"Console"</a>
                       <a href="/apps"
                            class={
                                if page_path.get().ends_with("/apps") {
                                    "bg-green-200 text-black border-b-2 font-medium font-mono border-black px-3 py-1 text-sm mr-4"
                                } else {
                                    "text-black hover:bg-green-200 font-mono hover:text-black px-3 py-1 text-sm mr-4"
                                }
                            }>"Apps"</a>

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
