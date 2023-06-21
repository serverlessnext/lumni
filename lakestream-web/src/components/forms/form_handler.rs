use leptos::ev::SubmitEvent;
use leptos::*;
use localencrypt::LocalEncrypt;

use super::form_load_handler::FormLoadVaultHandler;
use super::{
    FormContentView, FormLoadHandler, FormSubmitData, FormSubmitHandler,
    HtmlForm, SubmitInput,
};
use crate::components::form_helpers::FormSubmissionStatusView;
use crate::components::form_input::InputElements;

pub struct FormHandler {
    on_load: Option<Box<dyn FormLoadHandler>>,
    on_submit: Box<dyn FormSubmitHandler>,
}

impl FormHandler {
    pub fn new(
        on_load: Option<Box<dyn FormLoadHandler>>,
        on_submit: Box<dyn FormSubmitHandler>,
    ) -> Self {
        Self { on_load, on_submit }
    }

    pub fn new_with_vault(
        cx: Scope,
        form: HtmlForm,
        vault: &LocalEncrypt,
        submit_handler_factory: Box<
            dyn Fn(
                Scope,
                Option<&LocalEncrypt>,
                RwSignal<Option<FormSubmitData>>,
            ) -> Box<dyn FormSubmitHandler>,
        >,
    ) -> Self {
        let vault_handler = FormLoadVaultHandler::new(cx, form, vault);
        let form_data = vault_handler.form_data();
        let on_load: Option<Box<dyn FormLoadHandler>> = Some(vault_handler);

        let on_submit = submit_handler_factory(cx, Some(vault), form_data);

        Self { on_load, on_submit }
    }

    pub fn on_submit(&self) -> &dyn FormSubmitHandler {
        &*self.on_submit
    }

    pub fn on_load(&self) -> Option<&dyn FormLoadHandler> {
        self.on_load.as_ref().map(|handler| &**handler)
    }

    pub fn is_submitting(&self) -> RwSignal<bool> {
        self.on_submit.is_submitting()
    }

    pub fn submit_error(&self) -> RwSignal<Option<String>> {
        self.on_submit.submit_error()
    }
}

#[component]
pub fn SubmitFormView<'a>(
    cx: Scope,
    handler: &'a FormHandler,
    form_submit_data: FormSubmitData,
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
            />
            <FormSubmissionStatusView is_submitting={is_submitting.into()} submit_error={submit_error.into()} />
        </div>
    }
}
