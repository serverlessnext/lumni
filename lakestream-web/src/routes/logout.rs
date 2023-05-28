use leptos::{component, tracing, view, IntoView, Scope};
use crate::components::LogoutForm;

#[component]
pub fn Logout(cx: Scope) -> impl IntoView {
    view! { cx,
        <LogoutForm />
    }
}
