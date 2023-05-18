use leptos::*;
use leptos_meta::*;
use leptos_router::*;

use crate::base::GlobalState;
use crate::routes::{Home, ObjectStores, ObjectStoresId};

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    let state = create_rw_signal(cx, GlobalState::default());
    provide_context(cx, state);

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
                    </div>
                </nav>
                <main>
                    <Routes>
                        <Route path="/home" view=|cx| view! { cx, <Home/> }/>
                        <Route path="/object-stores" view=|cx| view! { cx, <ObjectStores/> }/>
                        <Route path="/object-stores/:id" view=|cx| view! { cx, <ObjectStoresId/> }/>
                    </Routes>
                </main>
            </Router>

        </div>
    }
}
