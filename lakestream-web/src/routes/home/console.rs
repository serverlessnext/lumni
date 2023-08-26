use leptos::*;

pub use crate::components::Environment;
pub use crate::external::objectstore_s3::App;

#[component]
pub fn Console(cx: Scope) -> impl IntoView {
    view! {
        cx,
        <Environment />
        <App />
        <br />
    }
}
