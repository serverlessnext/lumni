use leptos::*;
use leptos_router::{use_location, Outlet};

#[component]
pub fn Home(cx: Scope) -> impl IntoView {
    let page_path = use_location(cx).pathname;
    view! {
        cx,
        <nav class="bg-black">
            <div class="flex">
                {move ||
                    view! { cx,
                       <a href="/console"
                            class={
                                if page_path.get().ends_with("/console") {
                                    "bg-green-500/20 text-green-500 border-b-2 font-mono border-green-900 px-3 py-1 text-sm mr-4"
                                } else {
                                    "text-green-300 hover:bg-green-500/20 font-mono hover:text-green-100 px-3 py-1 text-sm mr-4"
                                }
                            }>"Console"</a>
                       <a href="/environment"
                            class={
                                if page_path.get().ends_with("/environment") {
                                    "bg-green-500/20 text-green-500 border-b-2 font-mono border-green-900 px-3 py-1 text-sm mr-4"
                                } else {
                                    "text-green-300 hover:bg-green-500/20 font-mono hover:text-green-100 px-3 py-1 text-sm mr-4"
                                }
                            }>"Environment"</a>

                    }
                }
            </div>
        </nav>

        <main>
            <Outlet />
        </main>
    }
}
