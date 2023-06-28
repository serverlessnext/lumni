use leptos::*;

pub use crate::components::SearchForm;
pub use crate::components::demo::{LoadFormDemo, LoadAndSubmitDemo};

#[component]
pub fn Home(cx: Scope) -> impl IntoView {
    view! {
        cx,
        <h1>"Home"</h1>
        <SearchForm />
        <br />
        // <h1>"Load Form Demo"</h1>
        // <LoadFormDemo />
        <h1>"Load and Submit Form Demo"</h1>
        <LoadAndSubmitDemo />
    }
}
