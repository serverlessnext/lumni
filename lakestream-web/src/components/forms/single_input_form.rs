use std::sync::Arc;

use leptos::ev::SubmitEvent;
use leptos::html::Input;
use leptos::*;

use crate::components::buttons::{FormSubmitButton, ButtonType};

#[derive(Clone)]
pub struct SingleInputForm {
    input_type: &'static str,
    input_placeholder: &'static str,
    button_type: ButtonType,
    pub on_submit: Arc<dyn Fn(SubmitEvent) + 'static>,
}

impl SingleInputForm {
    pub fn new(
        handle_submission: Arc<dyn Fn(SubmitEvent, bool)>,
        is_user_defined: bool,
        input_type: &'static str,
        input_placeholder: &'static str,
        button_type: ButtonType,
    ) -> Self {
        let on_submit = {
            let handle_submission = handle_submission.clone();
            move |ev: SubmitEvent| handle_submission(ev, is_user_defined)
        };

        SingleInputForm {
            input_type,
            input_placeholder,
            button_type,
            on_submit: Arc::new(on_submit),
        }
    }

    pub fn render_view(&self, cx: Scope, input_ref: NodeRef<Input>) -> View {
        let is_enabled = create_rw_signal(cx, true);

        view! {
            cx,
            <form class="flex flex-col w-96" on:submit={let cloned = Arc::clone(&self.on_submit); move |event| (cloned)(event)}>
                <div class="flex flex-col mb-4">
                    <input type={self.input_type}
                        placeholder={self.input_placeholder}
                        class="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
                        node_ref=input_ref
                    />
                </div>
                <div class="flex flex-col items-start">
                    <FormSubmitButton button_type=self.button_type.clone() button_enabled=is_enabled/>
                </div>

            </form>
        }.into_view(cx)
    }

}
