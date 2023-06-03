use leptos::{component, tracing, view, IntoView, Scope};

use crate::components::ChangePasswordForm;

#[component]
pub fn ChangePassword(cx: Scope) -> impl IntoView {
    view! { cx,
        <p>"Change Password Screen"</p>
        <ChangePasswordForm />
    }
}
