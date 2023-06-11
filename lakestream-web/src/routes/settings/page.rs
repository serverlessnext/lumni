use leptos::{component, tracing, view, IntoView, Scope};
use leptos_router::Outlet;

#[component]
pub fn Settings(cx: Scope) -> impl IntoView {
    view! { cx,
        <div>
            <nav>
                <div class="mb-2 text-white text-2xl font-bold">"Settings"</div>
                <div class="flex">
                    <a href="./user" class="text-teal-200 hover:text-white mr-4">"Other Settings"</a>
                    <a href="./change-password" class="text-teal-200 hover:text-white mr-4">"Change Password"</a>
                </div>
            </nav>
            <main>
                <Outlet />
            </main>
        </div>
    }
}
