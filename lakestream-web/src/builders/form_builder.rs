use leptos::ev::SubmitEvent;
use leptos::*;

use super::field_builder::FieldBuilderTrait;
use crate::components::buttons::FormButton;
use crate::components::form_input::FormElement;
use crate::components::forms::{Form, FormData, HtmlForm};

pub struct FormBuilder {
    title: String,
    id: String,
    elements: Vec<Box<dyn FieldBuilderTrait>>,
    form_type: FormType,
}

impl FormBuilder {
    pub fn new<S: Into<String>>(title: S, id: S, form_type: FormType) -> Self {
        Self {
            title: title.into(),
            id: id.into(),
            elements: Vec::new(),
            form_type,
        }
    }

    pub fn with_form_elements(
        mut self,
        form_elements: Vec<Box<dyn FieldBuilderTrait>>,
    ) -> Self {
        self.elements = form_elements;
        self
    }

    pub fn add_element(mut self, element: Box<dyn FieldBuilderTrait>) -> Self {
        self.elements.push(element);
        self
    }

    pub fn build(self, cx: Scope) -> Box<dyn Form> {
        let elements: Vec<FormElement> =
            self.elements.iter().map(|b| b.build()).collect();

        match self.form_type {
            FormType::SubmitData(parameters) => {
                HtmlForm::new(cx, &self.title, &self.id, elements)
                    .build(FormType::SubmitData(parameters))
            }
            FormType::LoadData(parameters) => {
                HtmlForm::new(cx, &self.title, &self.id, elements)
                    .build(FormType::LoadData(parameters))
            }
            FormType::LoadAndSubmitData(load_parameters, submit_parameters) => {
                HtmlForm::new(cx, &self.title, &self.id, elements).build(
                    FormType::LoadAndSubmitData(
                        load_parameters,
                        submit_parameters,
                    ),
                )
            }
            FormType::LoadElements => {
                HtmlForm::new(cx, &self.title, &self.id, elements)
                    .build(FormType::LoadElements)
            }
        }
    }
}

pub struct SubmitParameters {
    // pub parameters are meant to be consumed when used in a form
    pub submit_handler: Box<dyn Fn(SubmitEvent, Option<FormData>)>,
    pub form_button: Option<FormButton>,
    is_submitting: Option<RwSignal<bool>>,
    validation_error: Option<RwSignal<Option<String>>>,
}

impl SubmitParameters {
    pub fn new(
        submit_handler: Box<dyn Fn(SubmitEvent, Option<FormData>)>,
        is_submitting: Option<RwSignal<bool>>,
        validation_error: Option<RwSignal<Option<String>>>,
        form_button: Option<FormButton>,
    ) -> Self {
        Self {
            submit_handler,
            form_button,
            is_submitting,
            validation_error,
        }
    }

    pub fn is_submitting(&self) -> Option<RwSignal<bool>> {
        self.is_submitting
    }

    pub fn validation_error(&self) -> Option<RwSignal<Option<String>>> {
        self.validation_error
    }
}

pub struct LoadParameters {
    // pub parameters are meant to be consumed when used in a form
    pub load_handler: Option<Box<dyn Fn(RwSignal<Option<FormData>>)>>,
    is_loading: Option<RwSignal<bool>>,
    validation_error: Option<RwSignal<Option<String>>>,
}

impl LoadParameters {
    pub fn new(
        load_handler: Option<Box<dyn Fn(RwSignal<Option<FormData>>)>>,
        is_loading: Option<RwSignal<bool>>,
        validation_error: Option<RwSignal<Option<String>>>,
    ) -> Self {
        Self {
            load_handler,
            is_loading,
            validation_error,
        }
    }

    pub fn is_loading(&self) -> Option<RwSignal<bool>> {
        self.is_loading
    }

    pub fn validation_error(&self) -> Option<RwSignal<Option<String>>> {
        self.validation_error
    }
}

pub enum FormType {
    SubmitData(SubmitParameters),
    LoadData(LoadParameters),
    LoadAndSubmitData(LoadParameters, SubmitParameters),
    LoadElements,
}
