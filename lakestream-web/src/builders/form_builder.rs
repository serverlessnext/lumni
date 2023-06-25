use leptos::ev::SubmitEvent;
use leptos::*;

use crate::components::form_input::FormElement;
use super::field_builder::FieldBuilderTrait;
use crate::components::forms::{
    FormData, HtmlForm, LoadForm, SubmitForm, ViewCreator,
};

pub struct FormBuilder {
    title: String,
    id: String,
    elements: Vec<Box<dyn FieldBuilderTrait>>,
    submit_handler: Option<Box<dyn Fn(SubmitEvent, Option<FormData>)>>,
    is_submitting: Option<RwSignal<bool>>,
    validation_error: Option<RwSignal<Option<String>>>,
}

impl FormBuilder {
    pub fn new<S: Into<String>>(title: S, id: S) -> Self {
        Self {
            title: title.into(),
            id: id.into(),
            elements: Vec::new(),
            submit_handler: None,
            is_submitting: None,
            validation_error: None,
        }
    }

    pub fn add_element(mut self, element: Box<dyn FieldBuilderTrait>) -> Self {
        self.elements.push(element);
        self
    }

    pub fn on_submit(
        mut self,
        handler: Box<dyn Fn(SubmitEvent, Option<FormData>)>,
        is_submitting: RwSignal<bool>,
        validation_error: RwSignal<Option<String>>,
    ) -> Self {
        self.submit_handler = Some(handler);
        self.is_submitting = Some(is_submitting);
        self.validation_error = Some(validation_error);
        self
    }

    pub fn is_submitting(mut self, signal: RwSignal<bool>) -> Self {
        self.is_submitting = Some(signal);
        self
    }

    pub fn validation_error(
        mut self,
        signal: RwSignal<Option<String>>,
    ) -> Self {
        self.validation_error = Some(signal);
        self
    }

    pub fn build(self, cx: Scope) -> Box<dyn ViewCreator> {
        let elements: Vec<FormElement> =
            self.elements.iter().map(|b| b.build()).collect();

        let form = HtmlForm::new(&self.title, &self.id, elements);

        if let Some(submit_handler) = self.submit_handler {
            let form_handler = SubmitForm::new(
                cx,
                form,
                submit_handler,
                self.is_submitting
                    .unwrap_or_else(|| create_rw_signal(cx, false)),
                self.validation_error
                    .unwrap_or_else(|| create_rw_signal(cx, None)),
                None,
            );

            Box::new(form_handler)
        } else {
            let load_form = LoadForm::new(cx, form);
            Box::new(load_form)
        }
    }
}
