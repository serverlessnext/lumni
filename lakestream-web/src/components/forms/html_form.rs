use leptos::*;

use crate::builders::FormSubmitParameters;
use crate::components::form_input::FormElement;
use crate::components::forms::{FormData, LoadForm, SubmitForm};

pub enum FormType {
    Submit,
    Load,
}

pub struct HtmlForm {
    cx: Scope,
    html_form_meta: HtmlFormMeta,
    elements: Vec<FormElement>,
    submit_parameters: Option<FormSubmitParameters>,
    form_data_rw: RwSignal<Option<FormData>>,
}

impl HtmlForm {
    pub fn new(
        cx: Scope,
        name: &str,
        id: &str,
        elements: Vec<FormElement>,
        submit_parameters: Option<FormSubmitParameters>,
    ) -> Self {
        let form_data_rw = create_rw_signal(cx, None);
        let html_form_meta = HtmlFormMeta::new(name, id);

        Self {
            cx,
            html_form_meta,
            elements,
            submit_parameters,
            form_data_rw,
        }
    }

    pub fn build(self, form_type: FormType) -> Box<dyn Form> {
        match form_type {
            FormType::Submit => Box::new(SubmitForm::new(
                self.cx,
                self.html_form_meta,
                &self.elements,
                self.submit_parameters
                    .expect("Submit parameters are required"),
            )),
            FormType::Load => Box::new(LoadForm::new(self.cx, self)),
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

impl Form for HtmlForm {
    fn is_processing(&self) -> RwSignal<bool> {
        self.submit_parameters
            .as_ref()
            .and_then(|param| param.is_submitting())
            .unwrap_or_else(|| create_rw_signal(self.cx, false))
    }

    fn process_error(&self) -> RwSignal<Option<String>> {
        self.submit_parameters
            .as_ref()
            .and_then(|param| param.validation_error())
            .unwrap_or_else(|| create_rw_signal(self.cx, None))
    }

    fn form_data_rw(&self) -> RwSignal<Option<FormData>> {
        self.form_data_rw
    }

    fn to_view(&self) -> View {
        let cx = self.cx;
        view! {
            cx,
            <h1>{format!("Form: {}", self.html_form_meta.name())}</h1>
        }
        .into_view(cx)
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
