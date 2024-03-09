use leptos::ev::SubmitEvent;
use leptos::*;

use super::ElementBuilder;
use crate::components::buttons::FormButton;
use crate::components::forms::input::FormElement;
use crate::components::forms::{
    ConfigurationFormMeta, Form, FormData, FormViewOptions, HtmlForm,
};

pub struct FormBuilder {
    title: String,
    form_meta: ConfigurationFormMeta,
    elements: Vec<ElementBuilder>,
    form_type: FormType,
}

impl FormBuilder {
    pub fn new<S: Into<String>>(
        title: S,
        form_meta: ConfigurationFormMeta,
        form_type: FormType,
    ) -> Self {
        Self {
            title: title.into(),
            form_meta,
            elements: Vec::new(),
            form_type,
        }
    }

    pub fn with_elements<I, T>(mut self, form_elements: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<ElementBuilder>,
    {
        self.elements = form_elements.into_iter().map(Into::into).collect();
        self
    }

    pub fn get_elements(&self) -> &Vec<ElementBuilder> {
        &self.elements
    }

    pub fn clear_elements(&mut self) {
        self.elements.clear();
    }

    pub fn add_element<T: Into<ElementBuilder>>(
        &mut self,
        element: T,
    ) -> &mut Self {
        self.elements.push(element.into());
        self
    }

    pub fn build(self, view_options: Option<FormViewOptions>) -> Box<dyn Form> {
        let elements: Vec<FormElement> =
            self.elements.iter().map(|b| b.clone().build()).collect();

        match self.form_type {
            FormType::SubmitData(parameters) => HtmlForm::new(
                &self.title,
                self.form_meta,
                view_options,
                elements,
            )
            .build(FormType::SubmitData(parameters)),
            FormType::LoadData(parameters) => HtmlForm::new(
                &self.title,
                self.form_meta,
                view_options,
                elements,
            )
            .build(FormType::LoadData(parameters)),
            FormType::LoadAndSubmitData(load_parameters, submit_parameters) => {
                HtmlForm::new(
                    &self.title,
                    self.form_meta,
                    view_options,
                    elements,
                )
                .build(FormType::LoadAndSubmitData(
                    load_parameters,
                    submit_parameters,
                ))
            }
            FormType::LoadElements => HtmlForm::new(
                &self.title,
                self.form_meta,
                view_options,
                elements,
            )
            .build(FormType::LoadElements),
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
    //is_loading: Option<RwSignal<bool>>,
    //validation_error: Option<RwSignal<Option<String>>>,
}

impl LoadParameters {
    pub fn new(
        load_handler: Option<Box<dyn Fn(RwSignal<Option<FormData>>)>>,
    ) -> Self {
        Self { load_handler }
    }
}

pub enum FormType {
    SubmitData(SubmitParameters),
    LoadData(LoadParameters),
    LoadAndSubmitData(LoadParameters, SubmitParameters),
    LoadElements,
}
