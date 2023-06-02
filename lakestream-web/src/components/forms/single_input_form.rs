use std::sync::Arc;

use leptos::ev::SubmitEvent;
use leptos::html::Input;
use leptos::*;

use crate::components::forms::form_handler::{InputData, InputField};
use crate::components::buttons::{FormSubmitButton, ButtonType};


#[derive(Clone)]
pub struct SingleInputForm<T> {
    input_data: InputData,
    button_type: ButtonType,
    on_submit: Arc<dyn Fn(SubmitEvent, T) + 'static>,
    submission_data: T,
}

impl<T: Clone + 'static> SingleInputForm<T> {
    pub fn new(
        handle_submission: Arc<dyn Fn(SubmitEvent, T)>,
        submission_data: T,
        button_type: ButtonType,
        input_data: InputData,
    ) -> Self {
        SingleInputForm {
            input_data,
            button_type,
            on_submit: handle_submission,
            submission_data,
        }
    }

    pub fn render_view(&self, cx: Scope, input_ref: NodeRef<Input>) -> View {
        let is_enabled = create_rw_signal(cx, true);
        let validation_error = create_rw_signal(cx, None);

        let input_type = match self.input_data.input_field {
            InputField::Text { .. } => "text",
            InputField::Secret { .. } => "password",
            InputField::Password { .. } => "password",
        };

        let on_submit = {
            let handle_submission = Arc::clone(&self.on_submit);
            let submission_data = self.submission_data.clone();
            let validator = self.input_data.validator.clone();
            let input_ref = input_ref.clone();
            move |ev: SubmitEvent| {
                if let Some(validator) = &validator {
                    let value = input_ref().expect("input to exist").value();
                    if let Err(e) = validator(&value) {
                        validation_error.set(Some(e.to_string()));
                        ev.prevent_default();
                        return;
                    }
                }
                validation_error.set(None);
                handle_submission(ev, submission_data.clone());
            }
        };

        view! {
            cx,
            <form class="flex flex-col w-96" on:submit=on_submit>
                <div class="flex flex-col mb-4">
                    <input type={input_type}
                        value={&self.input_data.value}
                        class="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
                        node_ref=input_ref
                    />
                    { move || if let Some(error) = validation_error.get() {
                        view! {
                            cx,
                            <div class="text-red-500">
                                {error}
                            </div>
                        }
                    } else {
                        view! {
                            cx,
                            <div></div>
                        }
                    }}
                </div>
                <div class="flex flex-col items-start">
                    <FormSubmitButton button_type=self.button_type.clone() button_enabled=is_enabled/>
                </div>

            </form>
        }.into_view(cx)
    }
}

