use std::rc::Rc;

use leptos::ev::SubmitEvent;
use leptos::*;

use super::form_data::{FormData, SubmitInput};

pub trait SubmitHandler {
    fn data(&self) -> RwSignal<Option<FormData>>;
    fn on_submit(
        &self,
    ) -> Rc<dyn Fn(SubmitEvent, Option<SubmitInput>) + 'static>;
    fn is_submitting(&self) -> RwSignal<bool>;
    fn submit_error(&self) -> RwSignal<Option<String>>;
}

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
