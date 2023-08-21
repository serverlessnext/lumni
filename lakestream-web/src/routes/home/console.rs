use leptos::*;

pub use crate::components::SearchForm;

#[component]
pub fn Console(cx: Scope) -> impl IntoView {
    view! {
        cx,
        <SearchForm />
        <br />
    }
}
