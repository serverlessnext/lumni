use std::collections::HashMap;
use std::rc::Rc;

use leptos::*;
use localencrypt::{ItemMetaData, LocalEncrypt};

use super::form_data::FormData;
use super::html_form::HtmlForm;

const INVALID_BROWSER_STORAGE_TYPE: &str = "Invalid browser storage type";
const INVALID_STORAGE_BACKEND: &str = "Invalid storage backend";

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

impl LoadVaultHandler {
    pub fn new(cx: Scope, form: HtmlForm, vault: &LocalEncrypt) -> Box<Self> {
        let is_loading = create_rw_signal(cx, true);
        let load_error = create_rw_signal(cx, None::<String>);
        let form_data = create_rw_signal(cx, None::<FormData>);

        let form = Rc::new(form);
        let vault_clone = vault.clone();

        spawn_local(async move {
            match get_form_data_from_vault(cx, &form, &vault_clone).await {
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

pub async fn get_form_data_from_vault(
    cx: Scope,
    form: &HtmlForm,
    vault: &LocalEncrypt,
) -> Result<FormData, String> {
    let form_elements = form.elements.clone();
    let form_id = form.id();

    let mut tags = HashMap::new();
    tags.insert("Name".to_string(), form.name().to_string());
    let meta_data = ItemMetaData::new_with_tags(form_id, tags);

    let mut form_data = FormData::build(cx, meta_data, &form_elements);

    match load_config_from_vault(form_id, vault).await {
        Ok(Some(config)) => {
            form_data.update_with_config(config);
        }
        Ok(None) => {
            log::info!(
                "No data found for the given form id: {}. Using default \
                 configuration.",
                form_id
            );
        }
        Err(e) => {
            log::error!("error: {:?}", e);
            return Err(e);
        }
    };
    Ok(form_data)
}

async fn load_config_from_vault(
    form_id: &str,
    vault: &LocalEncrypt,
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
