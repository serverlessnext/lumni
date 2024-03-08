use leptos::*;

#[component]
pub fn SubmissionStatusView(
    is_submitting: Signal<bool>,
    submit_error: Signal<Option<String>>,
) -> impl IntoView {
    view! {
       // Show a loading message while the form is submitting
       { move || if is_submitting.get() {
           view! {
               <div>
                   "Submitting..."
               </div>
           }.into_view()
       } else {
           view! { }.into_view()
       }.into_view()}

       // Show an error message if there was an error during submission
       { move || if let Some(error) = submit_error.get() {
           view! {
               <div class="text-red-500">
                   {error}
               </div>
           }.into_view()
       } else {
           view! { }.into_view()
       }}
    }
}
