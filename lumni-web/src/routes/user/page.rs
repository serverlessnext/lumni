use leptos::*;
use leptos_router::{use_location, Outlet};

#[component]
pub fn User() -> impl IntoView {
    let page_path = use_location().pathname;
    view! {
        <nav class="bg-slate-100 mb-4">
            <div class="flex">
                {move ||
                    view! {
                        <a href="/user/settings"
                            class={
                                if page_path.get().ends_with("/user/settings") {
                                    "bg-green-200 text-black border-b-2 font-medium font-mono border-black px-3 py-1 text-sm mr-4"
                                } else {
                                    "text-black hover:bg-green-200 font-mono hover:text-black px-3 py-1 text-sm mr-4"
                                }
                            }>"Settings"</a>
                       <a href="/user/change-password"
                            class={
                                if page_path.get().ends_with("/change-password") {
                                    "bg-green-200 text-black border-b-2 font-medium font-mono border-black px-3 py-1 text-sm mr-4"
                                } else {
                                    "text-black hover:bg-green-200 font-mono hover:text-black px-3 py-1 text-sm mr-4"
                                }
                            }>"Change Password"</a>
                        <a href="/user/logout"
                            class={
                                if page_path.get().ends_with("/logout") {
                                    "bg-green-200 text-black border-b-2 font-medium font-mono border-black px-3 py-1 text-sm mr-4"
                                } else {
                                    "text-black hover:bg-green-200 font-mono hover:text-black px-3 py-1 text-sm mr-4"
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
