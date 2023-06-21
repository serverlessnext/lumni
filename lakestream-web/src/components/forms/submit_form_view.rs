use leptos::ev::SubmitEvent;
use leptos::*;

use super::form_content_view::FormContentView;
use super::form_data::{FormData, SubmitInput};
use super::handler::FormHandler;
use crate::components::buttons::ButtonType;
use crate::components::form_helpers::SubmissionStatusView;
use crate::components::form_input::InputElements;

#[component]
pub fn SubmitFormView<'a>(
    cx: Scope,
    handler: &'a FormHandler,
    form_submit_data: FormData,
    button_type: &'a ButtonType,
) -> impl IntoView {
    let is_submitting = handler.is_submitting();
    let submit_error = handler.submit_error();

    let input_elements = form_submit_data.input_elements();

    let rc_on_submit = handler.on_submit().on_submit();

    let box_on_submit: Box<dyn Fn(SubmitEvent, Option<InputElements>)> =
        Box::new(move |ev: SubmitEvent, elements: Option<InputElements>| {
            let elements = elements.map(SubmitInput::Elements);
            rc_on_submit(ev, elements);
        });

    view! {
        cx,
        <div>
            <FormContentView
                input_elements={input_elements}
                on_submit=box_on_submit
                is_submitting
                button_type
            />
            <SubmissionStatusView is_submitting={is_submitting.into()} submit_error={submit_error.into()} />
        </div>
    }
}
