use std::collections::HashMap;

use leptos::*;
use localencrypt::ItemMetaData;

use crate::builders::FormSubmitParameters;
use crate::components::form_input::{ElementData, ElementDataType, FormElement};
use crate::components::forms::{FormData, LoadForm, SubmitForm};

pub enum FormType {
    Submit,
    Load,
}

pub struct HtmlForm {
    cx: Scope,
    html_form_meta: HtmlFormMeta,
    form_data_rw: RwSignal<Option<FormData>>,
    submit_parameters: Option<FormSubmitParameters>,
}

impl HtmlForm {
    pub fn new(
        cx: Scope,
        name: &str,
        id: &str,
        elements: Vec<FormElement>,
        submit_parameters: Option<FormSubmitParameters>,
    ) -> Self {
        let mut tags = HashMap::new();
        tags.insert("Name".to_string(), name.to_string());
        let meta_data = ItemMetaData::new_with_tags(name, tags);

        let config = HashMap::new();
        let form_data =
            FormData::build(cx, meta_data, &config, &elements);
        let form_data_rw = create_rw_signal(cx, Some(form_data));

        let html_form_meta = HtmlFormMeta::new(name, id, elements);

        Self {
            cx,
            html_form_meta,
            form_data_rw,
            submit_parameters,
        }
    }

    pub fn build(self, form_type: FormType) -> Box<dyn Form> {
        match form_type {
            FormType::Submit => Box::new(SubmitForm::new(
                self.cx,
                self.html_form_meta,
                self.submit_parameters.expect("Submit parameters are required"),
            )),
            FormType::Load => {
                Box::new(LoadForm::new(self.cx, self.html_form_meta))
            }
        }
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
    elements: Vec<FormElement>,
}

impl HtmlFormMeta {
    pub fn new(name: &str, id: &str, elements: Vec<FormElement>) -> Self {
        Self {
            name: name.to_string(),
            id: id.to_string(),
            elements,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn elements(&self) -> Vec<FormElement> {
        self.elements.clone()
    }

    pub fn default_field_values(&self) -> HashMap<String, String> {
        self.elements
            .iter()
            .filter_map(|element| match element {
                FormElement::TextBox(element_data)
                | FormElement::TextArea(element_data)
                | FormElement::NestedForm(element_data) => get_default_value(element_data),
            })
            .collect()
    }
}


fn get_default_value(element_data: &ElementData) -> Option<(String, String)> {
    match &element_data.element_type {
        ElementDataType::TextData(text_data) => Some((
            element_data.name.clone(),
            text_data.buffer_data.clone(),
        )),
        ElementDataType::DocumentData(nested_form_data) => Some((
            element_data.name.clone(),
            nested_form_data.buffer_data.clone(),
        )),
        _ => None,
    }
}

