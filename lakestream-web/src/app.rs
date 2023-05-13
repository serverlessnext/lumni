use leptos::*;
use leptos_meta::*;
use leptos_router::*;

use crate::routes::home::{Home, HomeProps};
use crate::routes::config::{Config, ConfigProps};

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    view! {
        cx,
        <Stylesheet id="leptos" href="/pkg/tailwind.css"/>
        <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
        <div class="my-0 mx-auto px-8 max-w-7xl text-left">

            <Router>
                <nav>
                    <a href="/home">"Home"</a>
                    <a href="/config">"Config"</a>
                </nav>
                <main>
                    <Routes>
                        <Route path="/home" view=|cx| view! { cx, <Home/> }/>
                        <Route path="/config" view=|cx| view! { cx, <Config/> }/>
                    </Routes>
                </main>
            </Router>

        </div>
    }
}
