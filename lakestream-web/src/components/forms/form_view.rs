use std::collections::HashMap;

use leptos::ev::SubmitEvent;
use leptos::*;
use localencrypt::{DocumentMetaData, LocalEncrypt};

use super::form_handler::handle_form_submission;
use super::form_input::{
    create_input_elements, InputData, InputElements, InputFieldView,
};
use crate::components::buttons::{ButtonType, FormSubmitButton};

#[component]
pub fn FormView(
    cx: Scope,
    vault: LocalEncrypt,
    meta_data: DocumentMetaData,
    initial_config: HashMap<String, String>,
    default_config: HashMap<String, InputData>,
) -> impl IntoView {
    let (is_submitting, set_is_submitting) = create_signal(cx, false);
    let (submit_error, set_submit_error) = create_signal(cx, None::<String>);

    let input_elements =
        create_input_elements(cx, &initial_config, &default_config);
    let input_elements_clone_submit = input_elements.clone();

    let form_changed = create_rw_signal(cx, false);

    let on_submit = {
        move |ev: SubmitEvent, input_elements: InputElements| {
            ev.prevent_default(); // prevent form submission

            // Validate input elements
            let mut validation_errors = HashMap::new();

            for (key, (_, _, value_signal, _)) in &input_elements {
                let value = value_signal.get();
                let validator = default_config
                    .get(key)
                    .expect("Validator to exist")
                    .validator
                    .clone();

                if let Some(validator) = &validator {
                    if let Err(e) = validator(&value) {
                        log::error!("Validation failed: {}", e);
                        validation_errors.insert(key.clone(), e.to_string());
                    }
                }
            }

            // Write validation errors to corresponding WriteSignals
            for (key, (_, error_signal, _, _)) in &input_elements {
                if let Some(error) = validation_errors.get(key) {
                    error_signal.set(Some(error.clone()));
                } else {
                    error_signal.set(None);
                }
            }

            // If there are no validation errors, handle form submission
            if validation_errors.is_empty() {
                //let form_config = extract_config(&input_elements);
                let form_config: HashMap<String, String> = input_elements
                    .iter()
                    .map(|(key, (_, _, value_signal, _))| {
                        (key.clone(), value_signal.get())
                    })
                    .collect();

                let document_content =
                    serde_json::to_vec(&form_config).unwrap();

                handle_form_submission(
                    vault.clone(),
                    meta_data.clone(),
                    document_content,
                    set_is_submitting,
                    set_submit_error,
                );
            }
        }
    };

    view! {
        cx,
        <div>
            <form class="flex flex-wrap w-96"
                  on:submit=move |ev| {
                    set_is_submitting(true);
                    on_submit(ev, input_elements_clone_submit.clone())
                  }
            >
            <For
                each= move || {input_elements.clone().into_iter().enumerate()}
                    key=|(index, _input)| *index
                    view= move |cx, (_, (label, input_element))| {
                        view! {
                            cx,
                            <InputFieldView
                                label={label}
                                input_element={input_element}
                                input_changed={form_changed}
                            />
                        }

                    }
            />
            <FormSubmitButton button_type=ButtonType::Save(Some("Save Changes".to_string())) button_enabled=form_changed.into()/>
            </form>

        // Show a loading message while the form is submitting
        { move || if is_submitting.get() {
            view! {
                cx,
                <div>
                    "Submitting..."
                </div>
            }
        } else {
            view! {
                cx,
                <div></div>
            }
        }}

        // Show an error message if there was an error during submission
        { move || if let Some(error) = submit_error.get() {
            view! {
                cx,
                <div class="text-red-500">
                    {"Error during submission: "}
                    {error}
                </div>
            }
        } else {
            view! {
                cx,
                <div></div>
            }
        }}
        </div>
    }
}

pub trait OnSubmit {
    fn call(&mut self, ev: SubmitEvent, input_elements: InputElements);
}

impl<F: FnMut(SubmitEvent, InputElements)> OnSubmit for F {
    fn call(&mut self, ev: SubmitEvent, input_elements: InputElements) {
        self(ev, input_elements)
    }
}
