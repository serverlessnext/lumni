use leptos::*;

use crate::components::LoginForm;

#[component]
pub fn Login() -> impl IntoView {
    view! { <LoginForm />}.into_view()
}
