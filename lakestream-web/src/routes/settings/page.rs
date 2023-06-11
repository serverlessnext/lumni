use leptos::*;
use leptos_router::{Outlet, use_location};

#[component]
pub fn Settings(cx: Scope) -> impl IntoView {
    let page_path = use_location(cx).pathname;
    view! {
        cx,
        <nav class="bg-gradient-to-l from-gray-700 via-gray-900 to-black">
            <div class="flex">
                {move ||
                    view! { cx,
                        <a href="./user"
                            class={
                                if page_path.get().ends_with("/user") {
                                    "bg-teal-500/20 text-white border-b-2 font-medium border-teal-900 px-3 py-1 text-sm mr-4"
                                } else {
                                    "text-teal-200 hover:bg-teal-500/20 hover:text-teal-100 px-3 py-1 text-sm mr-4"
                                }
                            }>"Other Settings"</a>
                        <a href="./change-password"
                            class={
                                if page_path.get().ends_with("/change-password") {
                                    "bg-teal-500/20 text-white border-b-2 font-medium border-teal-900 px-3 py-1 text-sm mr-4"
                                } else {
                                    "text-teal-200 hover:bg-teal-500/20 hover:text-teal-100 px-3 py-1 text-sm mr-4"
                                }
                            }>"Change Password"</a>
                    }
                }
            </div>
        </nav>

        <main>
            <Outlet />
        </main>
    }
}

