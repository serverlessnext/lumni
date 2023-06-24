use leptos::*;

pub use crate::components::SearchForm;

#[component]
pub fn Home(cx: Scope) -> impl IntoView {
    view! {
        cx,
        <h1>"Home"</h1>
        <SearchForm />
    }
}
