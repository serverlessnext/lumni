use std::rc::Rc;

use leptos::ev::SubmitEvent;
use leptos::*;

use super::form_data::{FormData, SubmitInput};
use super::handler::FormHandlerTrait;
use crate::components::buttons::{FormButton, FormButtonGroup};
use crate::components::form_helpers::SubmissionStatusView;
use crate::components::form_input::{FormState, TextAreaView, TextBoxView};

pub struct ViewHandler {
    handler: Rc<dyn FormHandlerTrait>,
}

impl ViewHandler {
    pub fn new(handler: Rc<dyn FormHandlerTrait>) -> Self {
        Self { handler }
    }

    pub fn to_view(&self, cx: Scope, form_button: Option<FormButton>) -> View {
        let handler = Rc::clone(&self.handler);
        let is_processing_signal = handler.is_processing();
        let error_signal = handler.process_error();
        let form_data_signal = handler.form_data();

        view! { cx,
            {move ||
                if let Some(form_data) = form_data_signal.get() {
                    FormView(cx, &form_button, handler.clone(), form_data)
                }
                else if let Some(error) = error_signal.get() {
                    { ErrorView(cx, error) }.into_view(cx)
                }
                else if is_processing_signal.get() {
                    { SubmittingView(cx) }.into_view(cx)
                }
                else {
                    { LoadingView(cx) }.into_view(cx)
                }
            }
        }
        .into_view(cx)
    }
}

#[allow(non_snake_case)]
fn FormView(
    cx: Scope,
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
            SubmitFormView(cx, props).into_view(cx)
        }
        None => {
            let props = LoadFormViewProps { handler, form_data };
            LoadFormView(cx, props).into_view(cx)
        }
    }
}

#[allow(non_snake_case)]
fn LoadingView(cx: Scope) -> impl IntoView {
    view! {
        cx,
        <div>
            "Loading..."
        </div>
    }
}

#[allow(non_snake_case)]
fn SubmittingView(cx: Scope) -> impl IntoView {
    view! {
        cx,
        <div>
            "Submitting..."
        </div>
    }
}

#[allow(non_snake_case)]
fn ErrorView(cx: Scope, error: String) -> impl IntoView {
    view! {
        cx,
        <div>
            {"Error loading configuration: "}
            {error}
        </div>
    }
}

#[component]
fn SubmitFormView<'a>(
    cx: Scope,
    handler: Rc<dyn FormHandlerTrait>,
    form_data: FormData,
    form_button: &'a FormButton,
) -> impl IntoView {
    let is_submitting = handler.is_processing();
    let submit_error = handler.process_error();

    let form_state = form_data.form_state();

    let rc_on_submit = handler.on_submit().on_submit();
    let box_on_submit: Box<dyn Fn(SubmitEvent, Option<FormState>)> =
        Box::new(move |ev: SubmitEvent, elements: Option<FormState>| {
            let elements = elements.map(SubmitInput::Elements);
            rc_on_submit(ev, elements);
        });

    let mut button_group = FormButtonGroup::new(Some(true));
    button_group.add_button(form_button.clone());

    view! {
        cx,
        <div>
            <FormContentView
                form_state
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
    cx: Scope,
    handler: Rc<dyn FormHandlerTrait>,
    form_data: FormData,
) -> impl IntoView {
    let is_loading = handler.is_processing();
    let load_error = handler.process_error();

    let form_state = form_data.form_state();

    view! {
        cx,
        <h1>"Load Form"</h1>
        <form class="flex flex-wrap w-full max-w-2xl text-white border p-4 font-mono"
        >
            <For
                each= move || {form_state.clone().into_iter().enumerate()}
                    key=|(index, _)| *index
                    view= move |cx, (_, (_, form_element_state))| {
                        view! {
                            cx,
                            <TextAreaView
                                form_element_state
                            />
                        }
                    }
            />
        </form>
        <SubmissionStatusView is_submitting={is_loading.into()} submit_error={load_error.into()} />
    }.into_view(cx)
}

#[component]
pub fn FormContentView(
    cx: Scope,
    form_state: FormState,
    on_submit: Box<dyn Fn(SubmitEvent, Option<FormState>)>,
    is_submitting: RwSignal<bool>,
    buttons: FormButtonGroup,
) -> impl IntoView {
    let form_state_clone = form_state.clone();
    let form_changed = create_rw_signal(cx, false);
    view! {
        cx,
        <form class="flex flex-wrap w-full max-w-2xl text-black border p-4 font-mono"
            on:submit=move |ev| {
                is_submitting.set(true);
                on_submit(ev, Some(form_state.clone()))
            }
        >
            <For
                each= move || {form_state_clone.clone().into_iter().enumerate()}
                    key=|(index, _)| *index
                    view= move |cx, (_, (_, form_element_state))| {
                        view! {
                            cx,
                            <TextBoxView
                                form_element_state
                                input_changed={form_changed}
                            />
                        }
                    }
            />
            { move || buttons.clone().into_view(cx, Some(form_changed.get())) }
        </form>
    }.into_view(cx)
}
