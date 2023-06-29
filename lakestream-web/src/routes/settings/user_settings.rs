use std::collections::HashMap;

use leptos::ev::SubmitEvent;
use leptos::*;
use localencrypt::{ItemMetaData, LocalEncrypt};
use uuid::Uuid;

use crate::builders::{
    FieldBuilder, FormBuilder, FormType, LoadParameters, SubmitParameters,
};
use crate::components::form_input::{DisplayValue, ElementDataType, FormState};
use crate::components::forms::{FormData, FormError};
use crate::GlobalState;

const INVALID_BROWSER_STORAGE_TYPE: &str = "Invalid browser storage type";
const INVALID_STORAGE_BACKEND: &str = "Invalid storage backend";

const FORM_ID: &str = "user_settings";

#[derive(Debug, PartialEq, Clone)]
pub struct RouteParams {
    id: String,
}

#[component]
pub fn UserSettings(cx: Scope) -> impl IntoView {
    let vault = use_context::<RwSignal<GlobalState>>(cx)
        .expect("state to have been provided")
        .with(|state| state.vault.clone())
        .expect("vault to have been initialized");

    let username = "admin".to_string(); // TODO: get this from vault

    let form_id = FORM_ID;

    let is_loading = create_rw_signal(cx, false);
    let load_error = create_rw_signal(cx, None::<String>);

    let vault_clone = vault.clone();
    let handle_load = move |form_data_rw: RwSignal<Option<FormData>>| {
        let vault = vault_clone.clone();
        is_loading.set(true);
        spawn_local(async move {
            match load_config_from_vault(&vault, form_id).await {
                Ok(Some(config)) => {
                    let mut form_data = form_data_rw.get_untracked().unwrap();
                    form_data.update_with_config(config);
                    form_data_rw.set(Some(form_data));
                    is_loading.set(false);
                }
                Ok(None) => {
                    log!("No data found for form_id: {}", form_id);
                    is_loading.set(false);
                }
                Err(e) => {
                    log!(
                        "Error loading data: {:?} for form_id: {}",
                        e,
                        form_id
                    );
                    load_error.set(Some(e));
                    is_loading.set(false);
                }
            }
        });
    };

    // Handle form submit
    let is_submitting = create_rw_signal(cx, false);
    let submit_error = create_rw_signal(cx, None::<String>);
    let handle_submit = move |ev: SubmitEvent, form_data: Option<FormData>| {
        ev.prevent_default();
        is_submitting.set(true);
        let vault = vault.clone();

        spawn_local(async move {
            if let Some(form_data) = form_data {
                let form_state = form_data.form_state().clone();
                let validation_errors = perform_validation(&form_state);
                if validation_errors.is_empty() {
                    log!("Form data is valid: {:?}", form_data);
                    let result = save_config_to_vault(&vault, &form_data).await;
                    match result {
                        Ok(_) => {
                            log!("Data submitted successfully");
                            is_submitting.set(false);
                        }
                        Err(e) => {
                            log!("Data submission failed: {:?}", e);
                            submit_error.set(Some(e.to_string()));
                        }
                    }
                } else {
                    log!("Form data is invalid");
                    log!("Validation errors: {:?}", validation_errors);
                }
            } else {
                log!("Form data is empty");
            }
            is_submitting.set(false);
        });
    };

    let load_parameters = LoadParameters::new(
        Some(Box::new(handle_load)),
        Some(is_loading),
        Some(load_error),
    );

    let submit_parameters = SubmitParameters::new(
        Box::new(handle_submit),
        Some(is_submitting),
        Some(submit_error),
        None,
    );

    let form = FormBuilder::new(
        &username,
        &Uuid::new_v4().to_string(),
        FormType::LoadAndSubmitData(load_parameters, submit_parameters),
    )
    .add_element(Box::new(
        FieldBuilder::new("field1").with_label("a").as_input_field(),
    ))
    .add_element(Box::new(
        FieldBuilder::new("field2").with_label("b").as_input_field(),
    ))
    .build(cx);

    form.to_view()
}

async fn load_config_from_vault(
    vault: &LocalEncrypt,
    form_id: &str,
) -> Result<Option<HashMap<String, String>>, String> {
    let local_storage = match vault.backend() {
        localencrypt::StorageBackend::Browser(browser_storage) => {
            browser_storage
                .local_storage()
                .unwrap_or_else(|| panic!("{}", INVALID_BROWSER_STORAGE_TYPE))
        }
        _ => panic!("{}", INVALID_STORAGE_BACKEND),
    };

    let content_result = local_storage.load_content(form_id).await;

    match content_result {
        Ok(Some(data)) => {
            match serde_json::from_slice::<HashMap<String, String>>(&data) {
                Ok(config) => Ok(Some(config)),
                Err(e) => {
                    log::error!("error deserializing config: {:?}", e);
                    Err(e.to_string())
                }
            }
        }
        Ok(None) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

async fn save_config_to_vault(
    vault: &LocalEncrypt,
    form_data: &FormData,
) -> Result<(), FormError> {
    // convert form data to a HashMap<String, String>
    let form_state = form_data.form_state().clone();
    let form_config: HashMap<String, String> = form_state
        .iter()
        .map(|(key, element_state)| {
            (key.clone(), match element_state.read_display_value() {
                DisplayValue::Text(text) => text,
                _ => unreachable!(), // We've checked for Binary data above, so this should never happen
            })
        })
        .collect();

    // Serialize form data into JSON
    let document_content = match serde_json::to_vec(&form_config) {
        Ok(content) => content,
        Err(e) => {
            log::error!("error serializing config: {:?}", e);
            return Err(FormError::SubmitError(e.to_string()));
        }
    };

    // Get the local storage from the vault
    let mut local_storage = match vault.backend() {
        localencrypt::StorageBackend::Browser(browser_storage) => {
            browser_storage
                .local_storage()
                .unwrap_or_else(|| panic!("{}", INVALID_BROWSER_STORAGE_TYPE))
        }
        _ => panic!("{}", INVALID_STORAGE_BACKEND),
    };

    // Save the serialized form data to the local storage
    // TODO: should be obtained from the form data
    // let meta_data = form_data.meta_data().clone();
    let meta_data = ItemMetaData::new(FORM_ID);
    match local_storage
        .save_content(meta_data, &document_content)
        .await
    {
        Ok(_) => {
            log!("Successfully saved form data");
            Ok(())
        }
        Err(e) => {
            log!("Failed to save form data. Error: {:?}", e);
            Err(FormError::SubmitError(e.to_string()))
        }
    }
}

fn perform_validation(form_state: &FormState) -> HashMap<String, String> {
    let mut validation_errors = HashMap::new();
    for (key, element_state) in form_state {
        let value = element_state.read_display_value();
        let validator = match &element_state.schema.element_type {
            ElementDataType::TextData(text_data) => text_data.validator.clone(),
            // Add other ElementDataType cases if they have a validator
            _ => None,
        };

        if let Some(validator) = validator {
            match &value {
                DisplayValue::Text(text) => {
                    if let Err(e) = validator(text) {
                        log::error!("Validation failed: {}", e);
                        validation_errors.insert(key.clone(), e.to_string());
                    }
                }
                DisplayValue::Binary(_) => {
                    log::error!(
                        "Validation failed: Binary data cannot be validated."
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
