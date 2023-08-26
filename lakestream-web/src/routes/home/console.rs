use leptos::*;

pub use crate::components::{Environment, SearchForm};

#[component]
pub fn Console(cx: Scope) -> impl IntoView {
    view! {
        cx,
        <Environment />
        <SearchForm />
        <br />
    }
}
