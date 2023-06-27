use leptos::*;

pub use crate::components::{SearchForm, LoadDemo};

#[component]
pub fn Home(cx: Scope) -> impl IntoView {
    view! {
        cx,
        <h1>"Home"</h1>
        <SearchForm />
        <br />
        <h1>"Load Demo"</h1>
        <LoadDemo />
    }
}
