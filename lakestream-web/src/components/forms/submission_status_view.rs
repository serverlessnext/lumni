
use leptos::*;

#[component]
pub fn FormSubmissionStatusView(
    cx: Scope,
    is_submitting: ReadSignal<bool>,
    submit_error: ReadSignal<Option<String>>,
) -> impl IntoView {
    view! {
        cx,
        // Show a loading message while the form is submitting
        { move || if is_submitting.get() {
            view! {
                cx,
                <div>
                    "Submitting..."
                </div>
            }.into_view(cx)
        } else {
            view! { cx, }.into_view(cx)
        }.into_view(cx)}

        // Show an error message if there was an error during submission
        { move || if let Some(error) = submit_error.get() {
            view! {
                cx,
                <div class="text-red-500">
                    {"Error during submission: "}
                    {error}
                </div>
            }.into_view(cx)
        } else {
            view! { cx, }.into_view(cx)
        }}
     }
}
