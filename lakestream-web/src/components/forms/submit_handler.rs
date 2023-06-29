use std::rc::Rc;

use leptos::ev::SubmitEvent;
use leptos::*;
use localencrypt::LocalEncrypt;

use super::form_data::{FormData, SubmitInput};
use super::handler::FormHandlerTrait;
use super::html_form::HtmlForm;
use super::load_handler::{LoadHandler, LoadVaultHandler};

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
        form: HtmlForm,
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
