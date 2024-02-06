use leptos::*;

use crate::components::LoginForm;

#[component]
pub fn Login(cx: Scope) -> impl IntoView {
    view! { cx, <LoginForm />}.into_view(cx)
}
