use std::collections::HashMap;
use leptos::*;

use localencrypt::ItemMetaData;

use crate::builders::FormType;
use crate::components::form_input::FormElement;
use crate::components::forms::{FormData, LoadForm, SubmitForm, LoadAndSubmitForm};


pub struct HtmlForm {
    cx: Scope,
    html_form_meta: HtmlFormMeta,
    pub elements: Vec<FormElement>,
    form_data_rw: RwSignal<Option<FormData>>,
}

impl HtmlForm {
    pub fn new(
        cx: Scope,
        name: &str,
        id: &str,
        elements: Vec<FormElement>,
    ) -> Self {
        let html_form_meta = HtmlFormMeta::new(name, id);

        let form_data_rw = if !elements.is_empty() {
            let mut tags = HashMap::new();
            tags.insert("Name".to_string(), name.to_string());
            let meta_data = ItemMetaData::new_with_tags(id, tags);

            let form_data = FormData::build(cx, meta_data, &elements);
            create_rw_signal(cx, Some(form_data))
        } else {
            create_rw_signal(cx, None)
        };

        Self {
            cx,
            html_form_meta,
            elements,
            form_data_rw,
        }
    }

    pub fn build(self, form_type: FormType) -> Box<dyn Form> {
        match form_type {
            FormType::SubmitData(submit_parameters) => Box::new(SubmitForm::new(
                self,
                submit_parameters,
            )),
            FormType::LoadData(load_parameters) => {
                Box::new(LoadForm::new(self, Some(load_parameters)))
            }
            FormType::LoadElements => Box::new(LoadForm::new(self, None)),
            FormType::LoadAndSubmitData(load_parameters, submit_parameters) => {
                Box::new(LoadAndSubmitForm::new(
                    self,
                    load_parameters,
                    submit_parameters,
                ))
            }
        }
    }

    pub fn cx(&self) -> Scope {
        self.cx
    }

    pub fn name(&self) -> &str {
        &self.html_form_meta.name
    }

    pub fn id(&self) -> &str {
        &self.html_form_meta.id
    }

    pub fn form_data_rw(&self) -> RwSignal<Option<FormData>> {
        self.form_data_rw
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
}
