use std::collections::HashMap;

use leptos::ev::SubmitEvent;
use leptos::*;
use wasm_bindgen_futures::spawn_local;

use super::{
    create_input_elements, InputData, InputElements, InputFieldView,
    StringVault, FormOwner,
};

#[component]
pub fn FormView(
    cx: Scope,
    vault: StringVault,
    form_owner: FormOwner,
    initial_config: HashMap<String, String>,
    default_config: HashMap<String, InputData>,
) -> impl IntoView {
    let (is_submitting, set_is_submitting) = create_signal(cx, false);
    let (submit_error, set_submit_error) = create_signal(cx, None::<String>);

    let input_elements = create_input_elements(cx, &initial_config, &default_config);
    let input_elements_clone_submit = input_elements.clone();

    let on_submit = {
        move |ev: SubmitEvent, input_elements: InputElements| {
            ev.prevent_default(); // prevent form submission

            // Validate input elements
            let mut validation_errors = HashMap::new();

            for (key, (input_ref, _, _, _)) in &input_elements {
                let value = input_ref().expect("input to exist").value();
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
                log!("Validation successful");
                handle_form_submission(
                    vault.clone(),
                    form_owner.clone(),
                    input_elements,
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
                    view= move |cx, (_, (key, input_element))| {
                        view! {
                            cx,
                            <InputFieldView
                                key={key}
                                input_element={input_element}
                            />
                        }

                    }
            />
            <button
                type="submit"
                class="bg-amber-600 hover:bg-sky-700 px-5 py-3 text-white rounded-lg"
            >
                "Save"
            </button>
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

fn handle_form_submission(
    mut vault: StringVault,
    form_owner: FormOwner,
    input_elements: InputElements,
    set_is_submitting: WriteSignal<bool>,
    set_submit_error: WriteSignal<Option<String>>,
) {
    let config = extract_config(&input_elements);
    let form_id = form_owner.id.clone();
    spawn_local(async move {
        match vault.save_secure_configuration(form_owner, config.clone()).await {
            Ok(_) => {
                log!("Successfully saved secure configuration: {:?}", form_id);
                for (key, value) in &config {
                    if let Some((_, _, value_signal, _)) = input_elements.get(key)
                    {
                        value_signal.set(value.clone());
                    }
                }
                set_is_submitting.set(false);
            }
            Err(e) => {
                log!("Failed to save secure configuration. Error: {:?}", e);
                set_submit_error.set(Some(e.to_string()));
                set_is_submitting.set(false);
            }
        };
    });
    log!("Saved items");
}

fn extract_config(input_elements: &InputElements) -> HashMap<String, String> {
    let mut config: HashMap<String, String> = HashMap::new();
    for (key, (input_ref, _, value_writer, _)) in input_elements {
        let value = input_ref().expect("input to exist").value();
        config.insert(key.clone(), value.clone());
        value_writer.set(value);
    }
    config
}
