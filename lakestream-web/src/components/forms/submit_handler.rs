use std::collections::HashMap;
use std::rc::Rc;

use leptos::ev::SubmitEvent;
use leptos::*;
use localencrypt::{ItemMetaData, LocalEncrypt};

use super::form_data::{FormData, SubmitInput};
use super::handler::FormHandlerTrait;
use super::html_form::{Form, HtmlFormMeta};
use super::load_handler::{LoadHandler, LoadVaultHandler};
use super::view_handler::ViewHandler;
use crate::builders::FormSubmitParameters;
use crate::components::buttons::{ButtonType, FormButton};

type BoxedSubmitHandler = Box<
    dyn Fn(
        Scope,
        Option<&LocalEncrypt>,
        RwSignal<Option<FormData>>,
    ) -> Box<dyn SubmitHandler>,
>;

pub trait SubmitHandler {
    fn data(&self) -> RwSignal<Option<FormData>>;
    fn on_submit(
        &self,
    ) -> Rc<dyn Fn(SubmitEvent, Option<SubmitInput>) + 'static>;
    fn is_submitting(&self) -> RwSignal<bool>;
    fn submit_error(&self) -> RwSignal<Option<String>>;
}

#[derive(Clone)]
pub struct CustomSubmitHandler {
    on_submit: Rc<dyn Fn(SubmitEvent, Option<SubmitInput>) + 'static>,
    form_data: RwSignal<Option<FormData>>,
    is_submitting: RwSignal<bool>,
    submit_error: RwSignal<Option<String>>,
}

impl CustomSubmitHandler {
    pub fn new(
        form_data: RwSignal<Option<FormData>>,
        on_submit: Rc<dyn Fn(SubmitEvent, Option<SubmitInput>) + 'static>,
        is_submitting: RwSignal<bool>,
        submit_error: RwSignal<Option<String>>,
    ) -> Self {
        Self {
            on_submit,
            form_data,
            is_submitting,
            submit_error,
        }
    }
}

impl SubmitHandler for CustomSubmitHandler {
    fn data(&self) -> RwSignal<Option<FormData>> {
        self.form_data
    }

    fn on_submit(
        &self,
    ) -> Rc<dyn Fn(SubmitEvent, Option<SubmitInput>) + 'static> {
        self.on_submit.clone()
    }

    fn is_submitting(&self) -> RwSignal<bool> {
        self.is_submitting
    }

    fn submit_error(&self) -> RwSignal<Option<String>> {
        self.submit_error
    }
}

pub struct SubmitFormHandler {
    on_load: Option<Box<dyn LoadHandler>>,
    on_submit: Box<dyn SubmitHandler>,
}

impl SubmitFormHandler {
    pub fn new(
        on_load: Option<Box<dyn LoadHandler>>,
        on_submit: Box<dyn SubmitHandler>,
    ) -> Self {
        Self { on_load, on_submit }
    }

    pub fn new_with_vault(
        cx: Scope,
        form: HtmlFormMeta,
        vault: &LocalEncrypt,
        submit_handler: BoxedSubmitHandler,
    ) -> Self {
        let vault_handler = LoadVaultHandler::new(cx, form, vault);
        let form_data = vault_handler.form_data();
        let on_load: Option<Box<dyn LoadHandler>> = Some(vault_handler);

        let on_submit = submit_handler(cx, Some(vault), form_data);

        Self { on_load, on_submit }
    }

    pub fn on_submit(&self) -> &dyn SubmitHandler {
        &*self.on_submit
    }

    pub fn on_load(&self) -> Option<&dyn LoadHandler> {
        self.on_load.as_deref()
    }

    pub fn is_submitting(&self) -> RwSignal<bool> {
        self.on_submit.is_submitting()
    }

    pub fn submit_error(&self) -> RwSignal<Option<String>> {
        self.on_submit.submit_error()
    }
}

impl FormHandlerTrait for SubmitFormHandler {
    fn is_processing(&self) -> RwSignal<bool> {
        self.is_submitting()
    }

    fn process_error(&self) -> RwSignal<Option<String>> {
        self.submit_error()
    }

    fn form_data(&self) -> RwSignal<Option<FormData>> {
        self.on_submit().data()
    }
    fn on_submit(&self) -> &dyn SubmitHandler {
        self.on_submit()
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
        form: HtmlFormMeta,
        function: Box<dyn Fn(SubmitEvent, Option<FormData>) + 'static>,
        is_submitting: RwSignal<bool>,
        submit_error: RwSignal<Option<String>>,
        form_button: Option<FormButton>,
    ) -> Self {
        let default_field_values = form.default_field_values();
        let form_elements = form.elements();

        let mut tags = HashMap::new();
        tags.insert("Name".to_string(), form.name().to_string());
        let meta_data = ItemMetaData::new_with_tags(form.id(), tags);

        let form_data_default = FormData::build(
            cx,
            meta_data,
            &default_field_values,
            &form_elements,
        );

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

        let form_handler = Rc::new(SubmitFormHandler::new(
            None,
            custom_submit_handler
        ));
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

pub struct SubmitForm {
    cx: Scope,
    view_handler: ViewHandler,
    form_button: Option<FormButton>,
    is_processing: RwSignal<bool>,
    process_error: RwSignal<Option<String>>,
    form_data: RwSignal<Option<FormData>>,
}

impl SubmitForm {
    pub fn new(
        cx: Scope,
        form: HtmlFormMeta,
        submit_parameters: FormSubmitParameters,
    ) -> Self {
        let default_field_values = form.default_field_values();
        let form_elements = form.elements();

        let mut tags = HashMap::new();
        tags.insert("Name".to_string(), form.name().to_string());
        let meta_data = ItemMetaData::new_with_tags(form.id(), tags);

        let form_data_default = FormData::build(
            cx,
            meta_data,
            &default_field_values,
            &form_elements,
        );

        let is_processing = submit_parameters
            .is_submitting()
            .unwrap_or_else(|| create_rw_signal(cx, false));
        let process_error = submit_parameters
            .validation_error()
            .unwrap_or_else(|| create_rw_signal(cx, None));
        let form_button = submit_parameters.form_button;

        let form_data = create_rw_signal(cx, Some(form_data_default));
        let function = submit_parameters.submit_handler;
        let custom_submit_handler = Box::new(CustomSubmitHandler::new(
            form_data,
            Rc::new(
                move |ev: SubmitEvent, _submit_input: Option<SubmitInput>| {
                    function(ev, form_data.get());
                },
            ),
            is_processing,
            process_error,
        ));

        let form_handler =
            Rc::new(SubmitFormHandler::new(None, custom_submit_handler));
        let view_handler = ViewHandler::new(form_handler);

        Self {
            cx,
            view_handler,
            form_button,
            is_processing,
            process_error,
            form_data,
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

impl Form for SubmitForm {
    fn is_processing(&self) -> RwSignal<bool> {
        self.is_processing
    }

    fn process_error(&self) -> RwSignal<Option<String>> {
        self.process_error
    }

    fn form_data_rw(&self) -> RwSignal<Option<FormData>> {
        self.form_data
    }

    fn to_view(&self) -> View {
        self.to_view()
    }
}
