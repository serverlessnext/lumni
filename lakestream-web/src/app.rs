use leptos::*;
use leptos_meta::*;
use leptos_router::*;

use crate::routes::{Home, About, Login, ObjectStores, ObjectStoresId};
use crate::{GlobalState, RunTime};

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    let state = create_rw_signal(cx, GlobalState::default());
    provide_meta_context(cx);
    provide_context(cx, state);

    let set_previous_url =
        create_write_slice(cx, state, |state, previous_url| {
            state
                .runtime
                .get_or_insert_with(RunTime::new)
                .set_previous_url(previous_url);
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
                        <a href="/about" class="text-teal-200 hover:text-white mr-4">"About"</a>
                    </div>
                </nav>
                <main>
                    <Routes>
                        <Route path="/" view=|cx| view! { cx, <Home/> }/>
                        <Route path="/home" view=|cx| view! { cx, <Home/> }/>
                        <Route path="/about" view=|cx| view! { cx, <About/> }/>
                        <Route
                            path="/_login/:url"
                            view=move |cx| {
                                let location = use_location(cx);
                                let pathname = location.pathname.get();
                                let previous_path = pathname.strip_prefix("/_login").unwrap_or(&pathname).to_string();
                                set_previous_url(previous_path);
                                view! { cx, <Login/>}
                            }
                        />
                        <ProtectedRoute
                            path="/object-stores"
                            redirect_path="/_login/object-stores"
                            condition=move |_| vault_initialized.get()
                            view=|cx| view! { cx, <ObjectStores/> }
                        />
                        <ProtectedRoute
                            path="/object-stores/:id"
                            redirect_path="/_login/object-stores"
                            condition=move |_| vault_initialized.get()
                            view=|cx| view! { cx, <ObjectStoresId/> }/>
                    </Routes>
                </main>
            </Router>

        </div>
    }
}
