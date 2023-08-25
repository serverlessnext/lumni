use leptos::*;
use leptos_meta::*;
use leptos_router::*;

use crate::routes::api::Login;
use crate::routes::home::{Console, Home};
use crate::routes::home::apps::{Apps, AppId, AppConfiguration};
use crate::routes::user::{ChangePassword, Logout, User, UserSettings};
use crate::components::Redirect;
use crate::routes::About;
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
pub fn RedirectTo(cx: Scope, path: &'static str) -> impl IntoView {
    let navigate = use_navigate(cx);
    if let Err(e) = navigate(path, Default::default()) {
        log!("Error navigating to {}: {}", path, e);
    }
}

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    let state = create_rw_signal(cx, GlobalState::default());
    provide_meta_context(cx);
    provide_context(cx, state);

    let set_previous_url =
        create_write_slice(cx, state, |state, previous_url: String| {
            let updated_url = previous_url.replace(':', "/");
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
        <Stylesheet id="goaiio" href="/pkg/tailwind.css"/>
        <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
        <div class="my-0 mx-auto px-8 max-w-7xl text-left">
            <Router fallback=|cx| view! { cx, <Redirect redirect_url=None/>}.into_view(cx)>
                <nav class="py-2 px-4 text-lg font-medium h-24 bg-black">
                    <div class="mb-4 text-4xl font-sans font-bold bg-gradient-to-r from-white via-sky-200 to-sky-300 inline-block text-transparent bg-clip-text tracking-widest">"Goaiio"</div>
                    <div class="flex items-end text-white">
                        <a href="/console" class="hover:text-green-500 mr-4 font-mono font-bold">"Home"</a>
                        <a href="/user/settings" class="hover:text-green-500 mr-4 font-mono font-bold">"User"</a>
                        <a href="/about" class="hover:text-green-500 mr-4 font-mono font-bold">"About"</a>
                    </div>
                </nav>
                <main>
                    <Routes>
                        <Route
                            path=""
                            view=|cx| { view! { cx, <Home/>}}
                        >
                            // load Console directly if no path is given
                            // the url will be rewritten via History, saving
                            // a redirect on first page load
                            <Route path="/" view=|cx| view! { cx,
                                <Console />
                            }/>
                            // but also accept /console
                            <Route path="/console" view=|cx| view! { cx,
                                <Console />
                            }/>

                            <Route path="/apps" view=|cx| view! { cx,
                                <Apps />
                            }/>

                            <Route
                                path="/apps/:id"
                                view=move |cx| {
                                    if vault_initialized.get() == false {
                                        // not yet logged in
                                        let location = use_location(cx);
                                        let pathname = location.pathname.get();
                                        let redirect_url =
                                            format!("{}{}", redirect_path!(""),
                                            pathname.strip_prefix("/").unwrap_or_default().replace("/", ":")
                                        );
                                        view! {
                                            cx,
                                            <Redirect redirect_url=redirect_url.into()/>
                                        }.into_view(cx)
                                    }else {
                                        view! {
                                            cx,
                                            <AppConfiguration />
                                        }.into_view(cx)
                                    }
                                }

                            />
                            <Route
                                path="/apps/:_id/:id"
                                view=move |cx| {
                                    if vault_initialized.get() == false {
                                        // not yet logged in
                                        let location = use_location(cx);
                                        let pathname = location.pathname.get();
                                        let redirect_url =
                                            format!("{}{}", redirect_path!(""),
                                            pathname.strip_prefix("/").unwrap_or_default().replace("/", ":")
                                        );
                                        view! {
                                            cx,
                                            <Redirect redirect_url=redirect_url.into()/>
                                        }.into_view(cx)
                                    }else {
                                        view! {
                                            cx,
                                            <AppId />
                                        }.into_view(cx)
                                    }
                                }
                            />
                        </Route>
                        <ProtectedRoute
                            path="/user"
                            redirect_path=redirect_path!("user:settings")
                            condition=move |_| vault_initialized.get()
                            view=|cx| view! { cx, <User/> }
                        >
                            // catch /user, else fallback kicks in
                            <Route path="" view=|cx| view! { cx, <RedirectTo path="/user/settings"/> }/>
                           <Route path="settings" view=|cx| view! { cx,
                                <UserSettings />
                            }/>


                            <Route path="change-password" view=|cx| view! { cx,
                                <ChangePassword />
                            }/>
                        </ProtectedRoute>
                        <Route path="/about" view=|cx| view! { cx, <About/> }/>
                        <Route path="/user/logout" view=|cx| view! { cx, <Logout/> }/>
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
