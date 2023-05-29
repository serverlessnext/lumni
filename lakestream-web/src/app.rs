use leptos::*;
use leptos_meta::*;
use leptos_router::*;

use crate::routes::{
    About, Home, Login, Logout, ObjectStores, ObjectStoresId, UserId,
};
use crate::{GlobalState, RunTime};

// const API_PATH: &str = "/api/v1";

// while API_PATH const is preferred, cant use this at compile time in concat!
// redirect_path also must give a static string so cant use format"))
macro_rules! redirect_path {
    ($route:expr) => {
        concat!("/api/v1/login/", $route)
    };
}

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    let state = create_rw_signal(cx, GlobalState::default());
    provide_meta_context(cx);
    provide_context(cx, state);

    let set_previous_url =
        create_write_slice(cx, state, |state, previous_url: String| {
            let updated_url = previous_url.replace(":", "/");
            state
                .runtime
                .get_or_insert_with(RunTime::new)
                .set_previous_url(updated_url);
        });

    let vault_initialized = create_read_slice(cx, state, |state| {
        state
            .runtime
            .as_ref()
            .map(|r| r.vault_initialized())
            .unwrap_or_default()
    });

    view! {
        cx,
        <Stylesheet id="leptos" href="/pkg/tailwind.css"/>
        <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
        <div class="my-0 mx-auto px-8 max-w-7xl text-left">

            <Router>
                <nav class="bg-teal-500 p-3">
                    <div class="mb-2 text-white text-2xl font-bold">"Lakestream"</div>
                    <div class="flex">
                        <a href="/home" class="text-teal-200 hover:text-white mr-4">"Home"</a>
                        <a href="/object-stores" class="text-teal-200 hover:text-white mr-4">"ObjectStores"</a>
                        <a href="/users/admin" class="text-teal-200 hover:text-white mr-4">"Settings"</a>
                        <a href="/about" class="text-teal-200 hover:text-white mr-4">"About"</a>
                        <a href="/logout" class="text-teal-200 hover:text-white mr-4">"Logout"</a>
                    </div>
                </nav>
                <main>
                    <Routes>
                        <Route path="/" view=|cx| view! { cx, <Home/> }/>
                        <Route path="/home" view=|cx| view! { cx, <Home/> }/>
                        <ProtectedRoute
                            path="/object-stores"
                            redirect_path=redirect_path!("object-stores")
                            condition=move |_| vault_initialized.get()
                            view=|cx| view! { cx, <ObjectStores/> }
                        />
                        <ProtectedRoute
                            path="/object-stores/:id"
                            redirect_path=redirect_path!("object-stores")
                            condition=move |_| vault_initialized.get()
                            view=|cx| view! { cx, <ObjectStoresId/> }
                        />
                        <ProtectedRoute
                            path="/users/:id"
                            // we only support single admin id now
                            redirect_path=redirect_path!("users:admin")
                            condition=move |_| vault_initialized.get()
                            view=|cx| view! { cx, <UserId/> }
                        />
                        <Route path="/about" view=|cx| view! { cx, <About/> }/>
                        <Route path="/logout" view=|cx| view! { cx, <Logout/> }/>
                        <Route
                            path=redirect_path!(":id")
                            view=move |cx| {
                                let location = use_location(cx);
                                let pathname = location.pathname.get();
                                let previous_path = pathname.strip_prefix(redirect_path!("")).unwrap_or(&pathname).to_string();
                                set_previous_url(previous_path);
                                view! { cx, <Login/>}
                            }
                        />
                    </Routes>
                </main>
            </Router>

        </div>
    }
}
