use leptos::{component, tracing, view, IntoView, Scope};
use crate::components::login_form::LoginForm;


#[component]
pub fn Login(cx: Scope) -> impl IntoView {
    view! { cx,
        <LoginForm />
    }
}
