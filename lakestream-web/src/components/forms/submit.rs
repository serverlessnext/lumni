use std::collections::HashMap;
use std::rc::Rc;

use leptos::ev::SubmitEvent;
use leptos::*;

use super::html_form::{Form, HtmlForm};
use super::submit_handler::{CustomSubmitHandler, SubmitFormHandler};
use super::view_handler::ViewHandler;
use super::{ConfigurationFormMeta, FormData, SubmitInput};
use crate::builders::SubmitParameters;
use crate::components::buttons::{ButtonType, FormButton};

pub struct SubmitForm {
    form: HtmlForm,
    view_handler: ViewHandler,
    form_button: Option<FormButton>,
    is_processing: RwSignal<bool>,
    process_error: RwSignal<Option<String>>,
}

impl SubmitForm {
    pub fn new(form: HtmlForm, submit_parameters: SubmitParameters) -> Self {
        let cx = form.cx();
        let is_processing = submit_parameters
            .is_submitting()
            .unwrap_or_else(|| create_rw_signal(cx, false));
        let process_error = submit_parameters
            .validation_error()
            .unwrap_or_else(|| create_rw_signal(cx, None));

        let form_button = submit_parameters.form_button;

        let function = submit_parameters.submit_handler;
        let form_data_rw = form.form_data_rw();
        let custom_submit_handler = Box::new(CustomSubmitHandler::new(
            form_data_rw,
            Rc::new(
                move |ev: SubmitEvent, _submit_input: Option<SubmitInput>| {
                    function(ev, form_data_rw.get());
                },
            ),
            is_processing,
            process_error,
        ));

        let form_handler =
            Rc::new(SubmitFormHandler::new(custom_submit_handler));
        let view_handler = ViewHandler::new(form_handler);

        Self {
            form,
            view_handler,
            form_button,
            is_processing,
            process_error,
        }
    }

    pub fn to_view(&self) -> View {
        let form_button = self
            .form_button
            .clone()
            .unwrap_or(FormButton::new(ButtonType::Submit, None));
        self.view_handler.to_view(self.form.cx(), Some(form_button))
    }
}

impl Form for SubmitForm {
    fn is_processing(&self) -> RwSignal<bool> {
        self.is_processing
    }

    fn process_error(&self) -> RwSignal<Option<String>> {
        self.process_error
    }

    fn form_data_rw(&self) -> RwSignal<Option<FormData>> {
        self.form.form_data_rw()
    }

    fn to_view(&self) -> View {
        self.to_view()
    }
}

// this version of SubmitForm is still used by ChangePassWord and LoginForm
// which still must be restructured to use FormBuilder
pub struct SubmitFormClassic {
    cx: Scope,
    view_handler: ViewHandler,
    form_button: Option<FormButton>,
}

impl SubmitFormClassic {
    pub fn new(
        cx: Scope,
        form: HtmlForm,
        function: Box<dyn Fn(SubmitEvent, Option<FormData>) + 'static>,
        is_submitting: RwSignal<bool>,
        submit_error: RwSignal<Option<String>>,
        form_button: Option<FormButton>,
    ) -> Self {
        let mut tags = HashMap::new();
        tags.insert("Name".to_string(), form.name().to_string());

        let form_meta =
            ConfigurationFormMeta::with_id(form.id()).with_tags(tags);
        let form_data_default =
            FormData::build(cx, form_meta, &form.elements, None);

        let form_data = create_rw_signal(cx, Some(form_data_default));

        let custom_submit_handler = Box::new(CustomSubmitHandler::new(
            form_data,
            Rc::new(
                move |ev: SubmitEvent, _submit_input: Option<SubmitInput>| {
                    function(ev, form_data.get());
                },
            ),
            is_submitting,
            submit_error,
        ));

        let form_handler =
            Rc::new(SubmitFormHandler::new(custom_submit_handler));
        let view_handler = ViewHandler::new(form_handler);

        Self {
            cx,
            view_handler,
            form_button,
        }
    }

    pub fn to_view(&self) -> View {
        let form_button = self
            .form_button
            .clone()
            .unwrap_or(FormButton::new(ButtonType::Submit, None));
        self.view_handler.to_view(self.cx, Some(form_button))
    }
}
