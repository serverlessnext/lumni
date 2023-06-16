use std::collections::HashMap;

use leptos::ev::SubmitEvent;
use leptos::*;
use localencrypt::{ItemMetaData, LocalEncrypt};

use super::FormContentView;
use crate::components::form_helpers::FormSubmissionStatusView;
use crate::components::form_input::InputElements;

#[derive(Clone)]
pub struct FormSubmitData {
    input_elements: InputElements,
    meta_data: ItemMetaData,
}

impl FormSubmitData {
    pub fn new(input_elements: InputElements, meta_data: ItemMetaData) -> Self {
        Self {
            input_elements,
            meta_data,
        }
    }

    pub fn input_elements(&self) -> InputElements {
        self.input_elements.clone()
    }
}

pub struct FormSubmitHandler {
    vault: LocalEncrypt,
    data: FormSubmitData,
    is_submitting: RwSignal<bool>,
    submit_error: RwSignal<Option<String>>,
}

impl FormSubmitHandler {
    pub fn new(cx: Scope, vault: LocalEncrypt, data: FormSubmitData) -> Self {
        let is_submitting = create_rw_signal(cx, false);
        let submit_error = create_rw_signal(cx, None::<String>);

        Self {
            vault,
            data,
            is_submitting,
            submit_error,
        }
    }

    pub fn on_submit(&self) -> impl Fn(SubmitEvent, InputElements) {
        let vault = self.vault.clone();
        let meta_data = self.data.meta_data.clone();
        let is_submitting = self.is_submitting;
        let submit_error = self.submit_error;

        move |ev: SubmitEvent, input_elements: InputElements| {
            ev.prevent_default(); // prevent form submission

            // Validate input elements
            let mut validation_errors = HashMap::new();

            for (key, (_, _, value_signal, _)) in &input_elements {
                let value = value_signal.get();
                let validator = input_elements
                    .get(key)
                    .expect("Validator to exist")
                    .3
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
                    is_submitting,
                    submit_error,
                );
            }
        }
    }
    pub fn is_submitting(&self) -> RwSignal<bool> {
        self.is_submitting.clone()
    }

    pub fn submit_error(&self) -> RwSignal<Option<String>> {
        self.submit_error.clone()
    }
}

#[component]
pub fn SubmitFormView(cx: Scope, handler: FormSubmitHandler) -> impl IntoView {
    let input_elements = handler.data.input_elements();
    let is_submitting = handler.is_submitting();
    let submit_error = handler.submit_error();
    let on_submit = handler.on_submit();

    view! {
        cx,
        <div>
            <FormContentView
                input_elements={input_elements}
                on_submit={Box::new(on_submit)}
                is_submitting
            />
            <FormSubmissionStatusView is_submitting={is_submitting.into()} submit_error={submit_error.into()} />
        </div>
    }
}

pub fn handle_form_submission(
    vault: LocalEncrypt,
    meta_data: ItemMetaData,
    document_content: Vec<u8>,
    is_submitting: RwSignal<bool>,
    submit_error: RwSignal<Option<String>>,
) {
    let mut local_storage = match vault.backend() {
        localencrypt::StorageBackend::Browser(browser_storage) => {
            browser_storage
                .local_storage()
                .unwrap_or_else(|| panic!("Invalid browser storage type"))
        }
        _ => panic!("Invalid storage backend"),
    };

    spawn_local(async move {
        match local_storage
            .save_content(meta_data, &document_content)
            .await
        {
            Ok(_) => {
                log!("Successfully saved secure configuration",);
                is_submitting.set(false);
            }
            Err(e) => {
                log!("Failed to save secure configuration. Error: {:?}", e);
                submit_error.set(Some(e.to_string()));
                is_submitting.set(false);
            }
        };
    });
    log!("Saved items");
}
