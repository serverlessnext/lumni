use leptos::*;
use leptos_meta::*;
use leptos_router::*;

use crate::components::{ChangePasswordForm, Redirect};
use crate::routes::api::Login;
use crate::routes::configurations::{ConfigurationId, Configurations};
use crate::routes::{About, Home, Logout, Settings, UserSettings};
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
        <Stylesheet id="leptos" href="/pkg/tailwind.css"/>
        <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
        <div class="my-0 mx-auto px-8 max-w-7xl text-left">
            <Router fallback=|cx| view! { cx, <Redirect/>}.into_view(cx)>
                <nav class="py-2 px-4 text-lg font-medium h-24 bg-black">
                    <div class="mb-4 text-4xl font-sans font-bold bg-gradient-to-r from-white via-sky-200 to-sky-300 inline-block text-transparent bg-clip-text tracking-widest">"Goaiio"</div>
                    //<div class="mb-4 text-4xl font-sans tracking-widest font-bold text-white">"Goaiio"</div>
                    <div class="flex items-end text-white">
                        <a href="/console" class="hover:text-green-500 mr-4 font-mono font-bold">"Console"</a>
                        <a href="/user/user" class="hover:text-green-500 mr-4 font-mono font-bold">"User"</a>
                        <a href="/about" class="hover:text-green-500 mr-4 font-mono font-bold">"About"</a>
                    </div>
                </nav>
                <main>
                    <Routes>
                        <Route path="/" view=|cx| view! { cx, <Home/> }/>
                        <Route
                            path="/console"
                            view=|cx| view! { cx, <Home/> }
                        />
                        <ProtectedRoute
                            path="/user/profiles"
                            redirect_path=redirect_path!("user:profiles")
                            condition=move |_| vault_initialized.get()
                            view=|cx| view! { cx, <Configurations/> }
                        />
                        <ProtectedRoute
                            path="/user/profiles/:id"
                            redirect_path=redirect_path!("user:profiles")
                            condition=move |_| vault_initialized.get()
                            view=|cx| view! { cx, <ConfigurationId/> }
                        />
                        <ProtectedRoute
                            path="/user"
                            redirect_path=redirect_path!("user:user")
                            condition=move |_| vault_initialized.get()
                            view=|cx| view! { cx, <Settings/> }
                        >
                            // catch /user, else fallback kicks in
                            <Route path="" view=|cx| view! { cx, <RedirectTo path="/user/user"/> }/>

                            <Route path="user" view=|cx| view! { cx,
                                <UserSettings />
                            }/>
                            <Route path="change-password" view=|cx| view! { cx,
                                <p>"Change Password Screen"</p>
                                <ChangePasswordForm />
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
