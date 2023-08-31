use leptos::*;

pub use crate::components::Environment;

pub use crate::components::apps::AppFormSubmit;

#[component]
pub fn Console(cx: Scope) -> impl IntoView {
    view! {
        cx,
        <Environment />
        <AppFormSubmit />
        <br />
    }
}
