use std::collections::HashMap;
use std::rc::Rc;

use leptos::ev::SubmitEvent;
use leptos::*;
use localencrypt::{ItemMetaData, LocalEncrypt};

use super::form_data::{FormData, SubmitInput};
use crate::components::form_input::InputElements;

pub trait SubmitHandler {
    fn data(&self) -> RwSignal<Option<FormData>>;
    fn on_submit(
        &self,
    ) -> Rc<dyn Fn(SubmitEvent, Option<SubmitInput>) + 'static>;
    fn is_submitting(&self) -> RwSignal<bool>;
    fn submit_error(&self) -> RwSignal<Option<String>>;
}

pub struct SaveHandler {
    form_data: RwSignal<Option<FormData>>,
    is_submitting: RwSignal<bool>,
    submit_error: RwSignal<Option<String>>,
    on_submit_fn: Rc<dyn Fn(SubmitEvent, Option<SubmitInput>) + 'static>,
}

impl SaveHandler {
    pub fn new(
        cx: Scope,
        vault: &LocalEncrypt,
        form_data: RwSignal<Option<FormData>>,
    ) -> Box<Self> {
        let is_submitting = create_rw_signal(cx, false);
        let submit_error = create_rw_signal(cx, None::<String>);

        let form_data_clone = form_data;
        let vault = Rc::new(vault.clone());

        let on_submit_fn: Rc<dyn Fn(SubmitEvent, Option<SubmitInput>)> =
            Rc::new(
                move |submit_event: SubmitEvent, input: Option<SubmitInput>| {
                    let input_elements = match input {
                        Some(SubmitInput::Elements(elements)) => elements,
                        None => {
                            handle_no_form_data();
                            return;
                        }
                    };
                    let form_data = match form_data_clone.get() {
                        Some(data) => data,
                        None => {
                            handle_no_form_data();
                            return;
                        }
                    };

                    let vault = vault.clone();
                    let meta_data = form_data.meta_data().clone();
                    submit_event.prevent_default();

                    let validation_errors =
                        Self::perform_validation(&input_elements);

                    if validation_errors.is_empty() {
                        Self::submit_form(
                            &input_elements,
                            meta_data,
                            vault,
                            is_submitting.clone(),
                            submit_error.clone(),
                        );
                    }
                },
            );

        Box::new(Self {
            form_data,
            is_submitting,
            submit_error,
            on_submit_fn,
        })
    }

    fn perform_validation(
        input_elements: &InputElements,
    ) -> HashMap<String, String> {
        let mut validation_errors = HashMap::new();
        for (key, (_, _, value_signal, _)) in input_elements {
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
        for (key, (_, error_signal, _, _)) in input_elements {
            if let Some(error) = validation_errors.get(key) {
                error_signal.set(Some(error.clone()));
            } else {
                error_signal.set(None);
            }
        }
        validation_errors
    }

    fn submit_form(
        input_elements: &InputElements,
        meta_data: ItemMetaData,
        vault: Rc<LocalEncrypt>,
        is_submitting: RwSignal<bool>,
        submit_error: RwSignal<Option<String>>,
    ) {
        let form_config: HashMap<String, String> = input_elements
            .iter()
            .map(|(key, (_, _, value_signal, _))| {
                (key.clone(), value_signal.get())
            })
            .collect();
        let document_content = serde_json::to_vec(&form_config).unwrap();
        submit_save_form_content(
            vault,
            meta_data,
            document_content,
            is_submitting,
            submit_error,
        );
    }
}

fn handle_no_form_data() {
    log::warn!("Submit attempt made but form data is not available.");
}

impl SubmitHandler for SaveHandler {
    fn on_submit(
        &self,
    ) -> Rc<dyn Fn(SubmitEvent, Option<SubmitInput>) + 'static> {
        self.on_submit_fn.clone()
    }

    fn is_submitting(&self) -> RwSignal<bool> {
        self.is_submitting.clone()
    }

    fn submit_error(&self) -> RwSignal<Option<String>> {
        self.submit_error.clone()
    }

    fn data(&self) -> RwSignal<Option<FormData>> {
        self.form_data.clone()
    }
}

pub fn submit_save_form_content(
    vault: Rc<LocalEncrypt>,
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
