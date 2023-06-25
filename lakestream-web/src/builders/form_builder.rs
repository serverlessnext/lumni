use leptos::ev::SubmitEvent;
use leptos::*;

use super::field_builder::FieldBuilderTrait;
use crate::components::form_input::FormElement;
use crate::components::forms::{
    FormData, HtmlForm, LoadForm, SubmitForm, ViewCreator,
};

pub struct FormBuilder {
    title: String,
    id: String,
    elements: Vec<Box<dyn FieldBuilderTrait>>,
    form_parameters: Option<FormParameters>,
}

impl FormBuilder {
    pub fn new<S: Into<String>>(title: S, id: S) -> Self {
        Self {
            title: title.into(),
            id: id.into(),
            elements: Vec::new(),
            form_parameters: None,
        }
    }

    pub fn add_element(mut self, element: Box<dyn FieldBuilderTrait>) -> Self {
        self.elements.push(element);
        self
    }

    pub fn with_form_parameters(mut self, parameters: FormParameters) -> Self {
        self.form_parameters = Some(parameters);
        self
    }

    pub fn build(self, cx: Scope) -> Box<dyn ViewCreator> {
        let elements: Vec<FormElement> =
            self.elements.iter().map(|b| b.build()).collect();
        let form = HtmlForm::new(&self.title, &self.id, elements);

        if let Some(parameters) = self.form_parameters {
            let is_submitting = parameters
                .is_submitting
                .unwrap_or_else(|| create_rw_signal(cx, false));
            let validation_error = parameters
                .validation_error
                .unwrap_or_else(|| create_rw_signal(cx, None));

            let form_handler = SubmitForm::new(
                cx,
                form,
                parameters.submit_handler.unwrap(),
                is_submitting,
                validation_error,
                None,
            );

            Box::new(form_handler)
        } else {
            let load_form = LoadForm::new(cx, form);
            Box::new(load_form)
        }
    }
}

pub struct FormParameters {
    submit_handler: Option<Box<dyn Fn(SubmitEvent, Option<FormData>)>>,
    is_submitting: Option<RwSignal<bool>>,
    validation_error: Option<RwSignal<Option<String>>>,
}

impl FormParameters {
    pub fn new(
        submit_handler: Option<Box<dyn Fn(SubmitEvent, Option<FormData>)>>,
        is_submitting: Option<RwSignal<bool>>,
        validation_error: Option<RwSignal<Option<String>>>,
    ) -> Self {
        Self {
            submit_handler,
            is_submitting,
            validation_error,
        }
    }
}
