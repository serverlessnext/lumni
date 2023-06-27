use leptos::*;

use crate::builders::FormType;
use crate::components::form_input::FormElement;
use crate::components::forms::{FormData, LoadForm, SubmitForm};

pub struct HtmlForm {
    cx: Scope,
    html_form_meta: HtmlFormMeta,
    elements: Vec<FormElement>,
    form_data_rw: RwSignal<Option<FormData>>,
}

impl HtmlForm {
    pub fn new(
        cx: Scope,
        name: &str,
        id: &str,
        elements: Vec<FormElement>,
    ) -> Self {
        let form_data_rw = create_rw_signal(cx, None);
        let html_form_meta = HtmlFormMeta::new(name, id);

        Self {
            cx,
            html_form_meta,
            elements,
            form_data_rw,
        }
    }

    pub fn build(self, form_type: FormType) -> Box<dyn Form> {
        match form_type {
            FormType::Submit(parameters) => Box::new(SubmitForm::new(
                self.cx,
                self.html_form_meta,
                &self.elements,
                parameters,
            )),
            FormType::Load(parameters) => {
                if let Some(parameters) = parameters {
                    if let Some(handler) = parameters.load_handler {
                        // TODO: update LoadForm to accept parameters
                        log!("Handler passed");
                    }
                }
                Box::new(LoadForm::new(self.cx, self))
            }
        }
    }

    pub fn name(&self) -> &str {
        &self.html_form_meta.name
    }

    pub fn id(&self) -> &str {
        &self.html_form_meta.id
    }

    pub fn elements(&self) -> Vec<FormElement> {
        self.elements.clone()
    }
}

pub trait Form {
    fn is_processing(&self) -> RwSignal<bool>;
    fn process_error(&self) -> RwSignal<Option<String>>;
    fn form_data_rw(&self) -> RwSignal<Option<FormData>>;
    fn to_view(&self) -> View;
}

#[derive(Clone, Debug)]
pub struct HtmlFormMeta {
    name: String,
    id: String,
}

impl HtmlFormMeta {
    pub fn new(name: &str, id: &str) -> Self {
        Self {
            name: name.to_string(),
            id: id.to_string(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn id(&self) -> &str {
        &self.id
    }
}
