use leptos::*;

pub use crate::external::builders::AppFormSubmit;

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    view! {
        cx,
        <AppFormSubmit />
    }
}
