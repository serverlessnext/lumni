use leptos::*;

pub use crate::components::demo::{LoadAndSubmitDemo, LoadFormDemo};
pub use crate::components::SearchForm;

#[component]
pub fn Home(cx: Scope) -> impl IntoView {
    view! {
        cx,
        <h1>"Home"</h1>
        <SearchForm />
        <br />
        <h1>"Load Form Demo"</h1>
        <LoadFormDemo />
        <br />
        <h1>"Load and Submit Form Demo"</h1>
        <LoadAndSubmitDemo />
    }
}
