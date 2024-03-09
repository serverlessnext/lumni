use leptos::*;
use leptos_meta::*;
use leptos_router::*;

use crate::components::Redirect;
use crate::helpers::string_replace::replace_first_single_colon;
use crate::routes::api::Login;
use crate::routes::home::apps::uri::{AppId, AppUri};
use crate::routes::home::apps::Apps;
use crate::routes::home::{Console, Home};
use crate::routes::user::{ChangePassword, Logout, User, UserSettings};
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
pub fn RedirectTo(path: &'static str) -> impl IntoView {
    let navigate = use_navigate();
    navigate(path, Default::default());
}

#[component]
pub fn App() -> impl IntoView {
    let state = create_rw_signal(GlobalState::default());
    provide_meta_context();
    provide_context(state);

    let set_previous_url =
        create_write_slice(state, |state, previous_url: String| {
            //let updated_url = previous_url.replace(':', "/");
            let updated_url = replace_first_single_colon(&previous_url);
            state
                .runtime
                .get_or_insert_with(RunTime::new)
                .set_previous_url(updated_url);
        });

    let vault_initialized = create_read_slice(state, |state| state.is_vault_initialized());

    view! {
        <Stylesheet id="goaiio" href="/pkg/tailwind.css"/>
        <Link rel="shortcut icon" type_="image/ico" href="/favicon.ico"/>
        <div class="my-0 mx-auto px-8 max-w-7xl text-left">
            <Router fallback=|| view! { <Redirect redirect_url=None/>}.into_view()>
            <nav class="py-2 px-4 text-lg font-medium h-12 bg-customBlue flex items-center justify-between">
                <a href="/console" class="flex items-left">
                    <img src="/xlatti-logo-sm.png" alt="XLatti Logo" class="h-6 mr-1 mt-1" />
                    <div class="text-2xl font-mono font-light text-white inline-block text-transparent bg-clip-text tracking-widest">xlatti</div>
                </a>

                <div class="flex items-right text-white">
                    <a href="/user/settings" class="text-white hover:text-green-500 mr-3">
                        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke="currentColor" class="h-6 w-6">
                            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 6h16M4 12h16M4 18h16"></path>
                        </svg>
                    </a>
                    <a href="https://github.com/serverlessnext" target="_blank" class="text-white hover:text-green-500">
                        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16" fill="currentColor" class="h-5 w-5" aria-hidden="true">
                            <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.20-.82 2.20-.82.44 1.10.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.20 0 .21.15.46.55.38A8.013 8.013 0 0016 8c0-4.42-3.58-8-8-8z"/>
                        </svg>
                    </a>
                </div>
            </nav>

                <main>
                    <Routes>
                        <Route
                            path=""
                            view=|| { view! { <Home/>}}
                        >
                            // load Console directly if no path is given
                            // the url will be rewritten via History, saving
                            // a redirect on first page load
                            <Route path="/" view=|| view! { <Console /> }/>
                            // but also accept /console
                            <Route path="/console" view=|| view! { <Console /> }/>
                            <Route path="/apps" view=|| view! { <Apps /> }/>
                            <Route
                                path="/apps/:uri"
                                view=move || {
                                    if vault_initialized.get() == false {
                                        // not yet logged in
                                        let location = use_location();
                                        let pathname = location.pathname.get();
                                        let redirect_url =
                                            format!("{}{}", redirect_path!(""),
                                            pathname.strip_prefix("/").unwrap_or_default().replace("/", ":")
                                        );
                                        view! {
                                            <Redirect redirect_url=redirect_url.into()/>
                                        }.into_view()
                                    }else {
                                        view! {
                                            <AppUri />
                                        }.into_view()
                                    }
                                }

                            />
                            <Route
                                path="/apps/:uri/:id"
                                view=move || {
                                    if vault_initialized.get() == false {
                                        // not yet logged in
                                        let location = use_location();
                                        let pathname = location.pathname.get();
                                        let redirect_url =
                                            format!("{}{}", redirect_path!(""),
                                            pathname.strip_prefix("/").unwrap_or_default().replace("/", ":")
                                        );
                                        view! {
                                            <Redirect redirect_url=redirect_url.into()/>
                                        }.into_view()
                                    }else {
                                        view! { <AppId /> }.into_view()
                                    }
                                }
                            />
                        </Route>
                        <ProtectedRoute
                            path="/user"
                            redirect_path=redirect_path!("user:settings")
                            condition=move || vault_initialized.get()
                            view=|| view! { <User/> }
                        >
                            // catch /user, else fallback kicks in
                            <Route path="" view=|| view! { <RedirectTo path="/user/settings"/> }/>
                           <Route path="settings" view=|| view! { <UserSettings /> }/>


                            <Route path="change-password" view=|| view! { <ChangePassword /> }/>
                        </ProtectedRoute>
                        <Route path="/user/logout" view=|| view! { <Logout/> }/>
                        <Route
                            path=redirect_path!(":id")
                            view=move || {
                                let location = use_location();
                                let pathname = location.pathname.get();
                                let previous_path = pathname.strip_prefix(redirect_path!("")).unwrap_or(&pathname).to_string();
                                set_previous_url(previous_path);
                                view! { <Login/>}
                            }
                        />
                    </Routes>
                </main>
            </Router>
        </div>
    }
}
