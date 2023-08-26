use std::rc::Rc;

use leptos::*;

use super::handler::FormHandlerTrait;
use super::html_form::{Form, HtmlForm};
use super::submit_handler::SubmitHandler;
use super::view_handler::ViewHandler;
use super::FormData;
use crate::builders::LoadParameters;

pub struct LoadForm {
    form: HtmlForm,
    is_processing: RwSignal<bool>,
    process_error: RwSignal<Option<String>>,
}

impl LoadForm {
    pub fn new(form: HtmlForm, parameters: Option<LoadParameters>) -> Self {
        let is_processing = create_rw_signal(form.cx(), false);
        let process_error = create_rw_signal(form.cx(), None::<String>);

        if let Some(parameters) = parameters {
            if let Some(handler) = parameters.load_handler {
                // load handler writes to form_data_rw
                handler(form.form_data_rw());
            }
        }
        Self {
            form,
            is_processing,
            process_error,
        }
    }

    pub fn is_processing(&self) -> RwSignal<bool> {
        self.is_processing
    }

    pub fn process_error(&self) -> RwSignal<Option<String>> {
        self.process_error
    }

    pub fn form_data_rw(&self) -> RwSignal<Option<FormData>> {
        self.form.form_data_rw()
    }

    pub fn to_view(&self) -> View {
        let form_handler = LoadFormHandler {
            is_loading: self.is_processing,
            load_error: self.process_error,
            form_data: self.form_data_rw(),
        };
        ViewHandler::new(Rc::new(form_handler) as Rc<dyn FormHandlerTrait>)
            .to_view(self.form.cx(), None)
    }
}

impl Form for LoadForm {
    fn is_processing(&self) -> RwSignal<bool> {
        self.is_processing()
    }

    fn process_error(&self) -> RwSignal<Option<String>> {
        self.process_error()
    }

    fn form_data_rw(&self) -> RwSignal<Option<FormData>> {
        self.form_data_rw()
    }

    fn to_view(&self) -> View {
        self.to_view()
    }
}

pub struct LoadFormHandler {
    is_loading: RwSignal<bool>,
    load_error: RwSignal<Option<String>>,
    form_data: RwSignal<Option<FormData>>,
}

impl FormHandlerTrait for LoadFormHandler {
    fn is_processing(&self) -> RwSignal<bool> {
        self.is_loading
    }

    fn process_error(&self) -> RwSignal<Option<String>> {
        self.load_error
    }

    fn form_data(&self) -> RwSignal<Option<FormData>> {
        self.form_data
    }

    fn on_submit(&self) -> &dyn SubmitHandler {
        panic!(
            "LoadFormHandler might not have a submit handler, handle this \
             case appropriately"
        )
    }
}
