use std::collections::HashMap;
use std::rc::Rc;

use leptos::ev::SubmitEvent;
use leptos::*;
use localencrypt::{ItemMetaData, LocalEncrypt};

use super::form_data::{FormData, SubmitInput};
use super::html_form::HtmlForm;
use super::submit_handler::{SubmitFormHandler, SubmitHandler};
use super::view_handler::ViewHandler;
use crate::components::buttons::{ButtonType, FormButton};
use crate::components::form_input::{DisplayValue, ElementDataType, FormState};

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
                    let form_state = match input {
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
                        Self::perform_validation(&form_state);

                    if validation_errors.is_empty() {
                        Self::submit_form(
                            &form_state,
                            meta_data,
                            vault,
                            is_submitting,
                            submit_error,
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

    fn perform_validation(form_state: &FormState) -> HashMap<String, String> {
        let mut validation_errors = HashMap::new();
        for (key, element_state) in form_state {
            let value = element_state.read_display_value();
            let validator = match &element_state.schema.element_type {
                ElementDataType::TextData(text_data) => {
                    text_data.validator.clone()
                }
                // Add other ElementDataType cases if they have a validator
                _ => None,
            };

            if let Some(validator) = validator {
                match &value {
                    DisplayValue::Text(text) => {
                        if let Err(e) = validator(text) {
                            log::error!("Validation failed: {}", e);
                            validation_errors
                                .insert(key.clone(), e.to_string());
                        }
                    }
                    DisplayValue::Binary(_) => {
                        log::error!(
                            "Validation failed: Binary data cannot be \
                             validated."
                        );
                        validation_errors.insert(
                            key.clone(),
                            "Binary data cannot be validated.".to_string(),
                        );
                    }
                }
            }
        }

        // Write validation errors to corresponding WriteSignals
        for (key, element_state) in form_state {
            if let Some(error) = validation_errors.get(key) {
                element_state.display_error.set(Some(error.clone()));
            } else {
                element_state.display_error.set(None);
            }
        }
        validation_errors
    }

    fn submit_form(
        form_state: &FormState,
        meta_data: ItemMetaData,
        vault: Rc<LocalEncrypt>,
        is_submitting: RwSignal<bool>,
        submit_error: RwSignal<Option<String>>,
    ) {
        // Check for binary data
        for (_, element_state) in form_state.iter() {
            if let DisplayValue::Binary(_) = element_state.read_display_value()
            {
                log::error!(
                    "Form submission failed: Binary data detected in form \
                     data."
                );
                submit_error.set(Some(
                    "Binary data detected in form data.".to_string(),
                ));
                is_submitting.set(false);
                return;
            }
        }

        // Convert form data to string key/value pairs
        let form_config: HashMap<String, String> = form_state
            .iter()
            .map(|(key, element_state)| {
                (key.clone(), match element_state.read_display_value() {
                    DisplayValue::Text(text) => text,
                    _ => unreachable!(), // We've checked for Binary data above, so this should never happen
                })
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
        self.is_submitting
    }

    fn submit_error(&self) -> RwSignal<Option<String>> {
        self.submit_error
    }

    fn data(&self) -> RwSignal<Option<FormData>> {
        self.form_data
    }
}

pub struct SaveForm {
    cx: Scope,
    view_handler: ViewHandler,
}

impl SaveForm {
    pub fn new(cx: Scope, form: HtmlForm, vault: &LocalEncrypt) -> Self {
        let submit_handler = Box::new(
            move |_cx: Scope,
                  vault: Option<&LocalEncrypt>,
                  form_data: RwSignal<Option<FormData>>|
                  -> Box<dyn SubmitHandler> {
                // Ensure vault is available
                if let Some(_vault) = vault {
                    SaveHandler::new(_cx, _vault, form_data)
                } else {
                    panic!("Vault is required for SaveFormHandler");
                }
            },
        );

        let form_handler = Rc::new(SubmitFormHandler::new_with_vault(
            cx,
            form,
            vault,
            submit_handler,
        ));
        let view_handler = ViewHandler::new(form_handler);

        Self { cx, view_handler }
    }

    pub fn to_view(&self) -> View {
        let save_button =
            FormButton::new(ButtonType::Save, Some("Save Changes"))
                .set_enabled(false);
        self.view_handler.to_view(self.cx, Some(save_button))
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
