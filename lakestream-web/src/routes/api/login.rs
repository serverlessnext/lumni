use leptos::*;

use crate::components::{LoginForm, LoginFormDebug};

#[component]
pub fn Login(cx: Scope) -> impl IntoView {

    if cfg!(debug_assertions) {
        // view! { cx, <LoginFormDebug />}.into_view(cx)
        view! { cx, <LoginForm />}.into_view(cx)
    } else {
        view! { cx, <LoginForm />}.into_view(cx)
    }
}
