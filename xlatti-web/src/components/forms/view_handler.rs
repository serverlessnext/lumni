use std::rc::Rc;

use leptos::ev::SubmitEvent;
use leptos::*;

use super::handler::FormHandlerTrait;
use super::{FormData, FormElements, SubmitInput};
use crate::components::buttons::{FormButton, FormButtonGroup};
use crate::components::forms::helpers::SubmissionStatusView;
use crate::components::forms::input::TextBoxView;
use crate::components::forms::output::TextDisplayView;

pub struct ViewHandler {
    handler: Rc<dyn FormHandlerTrait>,
}

impl ViewHandler {
    pub fn new(handler: Rc<dyn FormHandlerTrait>) -> Self {
        Self { handler }
    }

    pub fn to_view(&self, form_button: Option<FormButton>) -> View {
        let handler = Rc::clone(&self.handler);
        let is_processing_signal = handler.is_processing();
        let error_signal = handler.process_error();
        let form_data_signal = handler.form_data();

        view! {
            {move ||
                if let Some(form_data) = form_data_signal.get() {
                    FormView(&form_button, handler.clone(), form_data)
                }
                else if let Some(error) = error_signal.get() {
                    { ErrorView(error) }.into_view()
                }
                else if is_processing_signal.get() {
                    { SubmittingView() }.into_view()
                }
                else {
                    { LoadingView() }.into_view()
                }
            }
        }
        .into_view()
    }
}

#[allow(non_snake_case)]
fn FormView(
    form_button: &Option<FormButton>,
    handler: Rc<dyn FormHandlerTrait>,
    form_data: FormData,
) -> View {
    match &form_button {
        Some(button) => {
            let props = SubmitFormViewProps {
                handler,
                form_data,
                form_button: button,
            };
            SubmitFormView(props).into_view()
        }
        None => {
            let props = LoadFormViewProps { handler, form_data };
            LoadFormView(props).into_view()
        }
    }
}

#[allow(non_snake_case)]
fn LoadingView() -> impl IntoView {
    view! {
        <div>
            "Loading..."
        </div>
    }
}

#[allow(non_snake_case)]
fn SubmittingView() -> impl IntoView {
    view! {
        <div>
            "Submitting..."
        </div>
    }
}

#[allow(non_snake_case)]
fn ErrorView(error: String) -> impl IntoView {
    view! {
        <div>
            {"Error loading configuration: "}
            {error}
        </div>
    }
}

#[component]
fn SubmitFormView<'a>(
    handler: Rc<dyn FormHandlerTrait>,
    form_data: FormData,
    form_button: &'a FormButton,
) -> impl IntoView {
    let is_submitting = handler.is_processing();
    let submit_error = handler.process_error();

    let rc_on_submit = handler.on_submit().on_submit();
    let box_on_submit: Box<dyn Fn(SubmitEvent, Option<FormElements>)> =
        Box::new(move |ev: SubmitEvent, elements: Option<FormElements>| {
            let elements = elements.map(SubmitInput::Elements);
            rc_on_submit(ev, elements);
        });

    let mut button_group = FormButtonGroup::new(Some(true));
    button_group.add_button(form_button.clone());

    view! {
        <div>
            <FormContentView
                form_data
                on_submit=box_on_submit
                is_submitting
                buttons=button_group
            />
            <SubmissionStatusView is_submitting={is_submitting.into()} submit_error={submit_error.into()} />
        </div>
    }
}

#[component]
fn LoadFormView(
    handler: Rc<dyn FormHandlerTrait>,
    form_data: FormData,
) -> impl IntoView {
    // ReadOnly Form
    let is_loading = handler.is_processing();
    let load_error = handler.process_error();

    let form_elements = form_data.elements().clone();

    view! {
        <form class="flex flex-wrap w-full max-w-2xl text-white border p-4 font-mono"
        >
            <For
                each= move || {form_elements.clone().into_iter().enumerate()}
                    key=|(index, _)| *index
                    children= move |(_, (_, form_element_state))| {
                        view! {
                            <TextDisplayView
                                form_element_state
                            />
                        }
                    }
            />
        </form>
        <SubmissionStatusView is_submitting={is_loading.into()} submit_error={load_error.into()} />
    }.into_view()
}

#[component]
pub fn FormContentView(
    form_data: FormData,
    on_submit: Box<dyn Fn(SubmitEvent, Option<FormElements>)>,
    is_submitting: RwSignal<bool>,
    buttons: FormButtonGroup,
) -> impl IntoView {
    let form_name = form_data.meta_data().name();
    let form_data_clone = form_data.clone();
    let form_changed = create_rw_signal(false);
    view! {
        <form class="flex flex-wrap w-full max-w-2xl text-black border p-4 font-mono"
            on:submit=move |ev| {
                is_submitting.set(true);
                on_submit(ev, Some(form_data.elements().to_owned()))
            }
        >
            {
                if let Some(name) = form_name {
                    view! {
                        <div class="w-full text-2xl">
                            {name}
                        </div>
                    }.into_view()
                } else {
                    "".into_view()
                }
            }
            <For
                each= move || {form_data_clone.elements().clone().into_iter().enumerate()}
                    key=|(index, _)| *index
                    children= move |(_, (_, form_element_state))| {
                        view! {
                            <TextBoxView
                                form_element_state
                                input_changed={form_changed}
                            />
                        }
                    }
            />
            { move || {
                buttons.clone().into_view(Some(form_changed.get())) }
            }
        </form>
    }.into_view()
}
