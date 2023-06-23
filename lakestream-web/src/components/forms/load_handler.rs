use std::collections::HashMap;

use leptos::*;
use localencrypt::{ItemMetaData, LocalEncrypt, SecureStringError};

use super::form_data::FormData;
use super::HtmlForm;
use crate::components::form_input::FormElement;

const INVALID_BROWSER_STORAGE_TYPE: &str = "Invalid browser storage type";
const INVALID_STORAGE_BACKEND: &str = "Invalid storage backend";
const CANT_LOAD_CONFIG: &str =
    "Can't load existing configuration. Creating new.";

pub trait LoadHandler {
    fn is_loading(&self) -> RwSignal<bool>;
    fn load_error(&self) -> RwSignal<Option<String>>;
    fn form_data(&self) -> RwSignal<Option<FormData>>;
}

pub struct LoadVaultHandler {
    form_data: RwSignal<Option<FormData>>,
    is_loading: RwSignal<bool>,
    load_error: RwSignal<Option<String>>,
}

impl LoadHandler for LoadVaultHandler {
    fn is_loading(&self) -> RwSignal<bool> {
        self.is_loading
    }

    fn load_error(&self) -> RwSignal<Option<String>> {
        self.load_error
    }

    fn form_data(&self) -> RwSignal<Option<FormData>> {
        self.form_data
    }
}

use std::rc::Rc;
impl LoadVaultHandler {
    pub fn new(cx: Scope, form: HtmlForm, vault: &LocalEncrypt) -> Box<Self> {
        let is_loading = create_rw_signal(cx, true);
        let load_error = create_rw_signal(cx, None::<String>);
        let form_data = create_rw_signal(cx, None::<FormData>);

        let form = Rc::new(form);
        let vault_clone = vault.clone();

        spawn_local(async move {
            match get_form_data(cx, &form, &vault_clone).await {
                Ok(form_submit_data) => {
                    form_data.set(Some(form_submit_data));
                    is_loading.set(false);
                }
                Err(error) => {
                    load_error.set(Some(error));
                    is_loading.set(false);
                }
            }
        });

        Box::new(Self {
            form_data,
            is_loading,
            load_error,
        })
    }
}

fn handle_loaded_content(
    cx: Scope,
    form_name: &str,
    form_elements: &[FormElement],
    meta_data: ItemMetaData,
    content: Result<Option<Vec<u8>>, SecureStringError>,
    default_field_values: &HashMap<String, String>,
) -> Result<FormData, String> {
    match content {
        Ok(data) => match data {
            Some(data) => match serde_json::from_slice(&data) {
                Ok(new_config) => {
                    let form_submit_data = FormData::create_from_elements(
                        cx,
                        meta_data,
                        &new_config,
                        form_elements,
                    );
                    Ok(form_submit_data)
                }
                Err(e) => {
                    log::error!("error deserializing config: {:?}", e);
                    Err(e.to_string())
                }
            },
            None => {
                log::info!(
                    "No data found for the given form id: {}. Creating new.",
                    form_name
                );
                let form_submit_data = FormData::create_from_elements(
                    cx,
                    meta_data,
                    default_field_values,
                    form_elements,
                );
                Ok(form_submit_data)
            }
        },
        Err(e) => match e {
            SecureStringError::PasswordNotFound(_)
            | SecureStringError::NoLocalStorageData => {
                log::info!("{} Creating new.", CANT_LOAD_CONFIG);
                let form_submit_data = FormData::create_from_elements(
                    cx,
                    meta_data,
                    default_field_values,
                    form_elements,
                );
                Ok(form_submit_data)
            }
            _ => {
                log::error!("error loading config: {:?}", e);
                Err(e.to_string())
            }
        },
    }
}

pub async fn get_form_data(
    cx: Scope,
    form: &HtmlForm,
    vault: &LocalEncrypt,
) -> Result<FormData, String> {
    let default_field_values = form.default_field_values();
    let form_elements = form.elements();
    let form_name = form.id();

    let mut tags = HashMap::new();
    tags.insert("Name".to_string(), form.name());
    let meta_data = ItemMetaData::new_with_tags(&form_name, tags);

    let content = fetch_form_data(&form_name, vault).await;
    handle_loaded_content(
        cx,
        &form_name,
        &form_elements,
        meta_data,
        content,
        &default_field_values,
    )
}

async fn fetch_form_data(
    form_name: &str,
    vault: &LocalEncrypt,
) -> Result<Option<Vec<u8>>, SecureStringError> {
    let local_storage = match vault.backend() {
        localencrypt::StorageBackend::Browser(browser_storage) => {
            browser_storage
                .local_storage()
                .unwrap_or_else(|| panic!("{}", INVALID_BROWSER_STORAGE_TYPE))
        }
        _ => panic!("{}", INVALID_STORAGE_BACKEND),
    };

    local_storage.load_content(form_name).await
}
