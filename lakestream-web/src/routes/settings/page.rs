use leptos::*;
use leptos_router::{use_location, Outlet};

#[component]
pub fn Settings(cx: Scope) -> impl IntoView {
    let page_path = use_location(cx).pathname;
    view! {
        cx,
        <nav class="bg-black">
            <div class="flex">
                {move ||
                    view! { cx,
                        <a href="./profiles"
                            class={
                                if page_path.get().ends_with("/profiles") {
                                    "bg-green-500/20 text-green-500 border-b-2 font-mono border-green-900 px-3 py-1 text-sm mr-4"
                                } else {
                                    "text-green-300 hover:bg-green-500/20 font-mono hover:text-green-100 px-3 py-1 text-sm mr-4"
                                }
                            }>"Profiles"</a>
                        <a href="./user"
                            class={
                                if page_path.get().ends_with("/user") {
                                    "bg-green-500/20 text-green-500 border-b-2 font-mono border-green-900 px-3 py-1 text-sm mr-4"
                                } else {
                                    "text-green-300 hover:bg-green-500/20 font-mono hover:text-green-100 px-3 py-1 text-sm mr-4"
                                }
                            }>"Other Settings"</a>
                        <a href="./change-password"
                            class={
                                if page_path.get().ends_with("/change-password") {
                                    "bg-green-500/20 text-green-500 border-b-2 font-mono border-green-900 px-3 py-1 text-sm mr-4"
                                } else {
                                    "text-green-300 hover:bg-green-500/20 font-mono hover:text-green-100 px-3 py-1 text-sm mr-4"
                                }
                            }>"Change Password"</a>
                        <a href="./logout"
                            class={
                                if page_path.get().ends_with("/logout") {
                                    "bg-green-500/20 text-green-500 border-b-2 font-mono border-green-900 px-3 py-1 text-sm mr-4"
                                } else {
                                    "text-green-300 hover:bg-green-500/20 font-mono hover:text-green-100 px-3 py-1 text-sm mr-4"
                                }
                            }>"Logout"</a>
                    }
                }
            </div>
        </nav>

        <main>
            <Outlet />
        </main>
    }
}
