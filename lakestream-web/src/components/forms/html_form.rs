use std::collections::HashMap;
use std::sync::Arc;

use leptos::*;
use localencrypt::{ItemMetaData, LocalEncrypt, SecureStringError};
use serde_json;
use wasm_bindgen_futures::spawn_local;

use super::form_submit::{FormSubmitData, FormSubmitHandler, SubmitFormView};
use crate::components::form_input::{InputData, InputElements};

const INVALID_BROWSER_STORAGE_TYPE: &str = "Invalid browser storage type";
const INVALID_STORAGE_BACKEND: &str = "Invalid storage backend";
const CANT_LOAD_CONFIG: &str =
    "Can't load existing configuration. Creating new.";

#[derive(Clone, Debug)]
pub struct HtmlForm {
    name: String,
    id: String,
    fields: HashMap<String, InputData>,
}

impl HtmlForm {
    pub fn new(
        name: &str,
        id: &str,
        fields: HashMap<String, InputData>,
    ) -> Self {
        Self {
            name: name.to_string(),
            id: id.to_string(),
            fields,
        }
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn id(&self) -> String {
        self.id.clone()
    }

    pub fn fields(&self) -> HashMap<String, InputData> {
        self.fields.clone()
    }

    pub fn default_field_values(&self) -> HashMap<String, String> {
        self.fields
            .iter()
            .map(|(key, input_data)| (key.clone(), input_data.value.clone()))
            .collect()
    }
}

pub struct HtmlFormHandler {
    form: HtmlForm,
    vault: LocalEncrypt,
}

impl HtmlFormHandler {
    pub fn new(form: HtmlForm, vault: LocalEncrypt) -> Self {
        Self { form, vault }
    }

    pub fn create_view(&self, cx: Scope) -> View {
        let (form_submit_data_signal, set_form_submit_data) =
            create_signal(cx, None::<FormSubmitData>);

        let (load_config_error, set_load_config_error) =
            create_signal(cx, None::<String>);

        let html_form = self.form.clone();

        let local_storage = match self.vault.backend() {
            localencrypt::StorageBackend::Browser(browser_storage) => {
                browser_storage.local_storage().unwrap_or_else(|| {
                    panic!("{}", INVALID_BROWSER_STORAGE_TYPE)
                })
            }
            _ => panic!("{}", INVALID_STORAGE_BACKEND),
        };

        create_effect(cx, move |_| {
            let default_field_values = html_form.default_field_values();
            let default_fields = html_form.fields();
            let form_name = html_form.id();
            let local_storage = local_storage.clone();

            let mut tags = HashMap::new();
            tags.insert("Name".to_string(), html_form.name());
            let meta_data = ItemMetaData::new_with_tags(&form_name, tags);

            spawn_local(async move {
                match local_storage.load_content(&form_name).await {
                    Ok(Some(data)) => match serde_json::from_slice(&data) {
                        Ok(new_config) => {
                            let form_submit_data = create_form_submit_data(
                                cx,
                                meta_data.clone(),
                                &new_config,
                                &default_fields,
                            );
                            set_form_submit_data(Some(form_submit_data));
                        }
                        Err(e) => {
                            log::error!("error deserializing config: {:?}", e);
                            set_load_config_error(Some(e.to_string()));
                        }
                    },
                    Ok(None) => {
                        log::info!(
                            "No data found for the given form id: {}. \
                             Creating new.",
                            &form_name
                        );
                        let form_submit_data = create_form_submit_data(
                            cx,
                            meta_data.clone(),
                            &default_field_values,
                            &default_fields,
                        );
                        set_form_submit_data(Some(form_submit_data));
                    }
                    Err(e) => match e {
                        SecureStringError::PasswordNotFound(_)
                        | SecureStringError::NoLocalStorageData => {
                            log::info!("{} Creating new.", CANT_LOAD_CONFIG);
                            let form_submit_data = create_form_submit_data(
                                cx,
                                meta_data.clone(),
                                &default_field_values,
                                &default_fields,
                            );
                            set_form_submit_data(Some(form_submit_data));
                        }
                        _ => {
                            log::error!("error loading config: {:?}", e);
                            set_load_config_error(Some(e.to_string()));
                        }
                    },
                }
            });
        });

        let vault = self.vault.clone();

        view! { cx,
            {move ||
                if let Some(form_submit_data) = form_submit_data_signal.get() {
                    let handler = FormSubmitHandler::new(cx, vault.clone(), form_submit_data.clone());
                    view! {
                        cx,
                        <div>
                            <SubmitFormView handler/>
                        </div>
                    }.into_view(cx)
                }
                else if let Some(error) = load_config_error.get() {
                    view! {
                        cx,
                        <div>
                            {"Error loading configuration: "}
                            {error}
                        </div>
                    }.into_view(cx)
                }
                else {
                    view! {
                        cx,
                        <div>
                            "Loading..."
                        </div>
                    }.into_view(cx)
                }
            }
        }.into_view(cx)
    }
}

fn create_form_submit_data(
    cx: Scope,
    meta_data: ItemMetaData,
    config: &HashMap<String, String>,
    default_fields: &HashMap<String, InputData>,
) -> FormSubmitData {
    let input_elements: InputElements = config
        .iter()
        .map(|(key, value)| {
            let error_signal = create_rw_signal(cx, None);
            let value_signal = create_rw_signal(cx, value.clone());
            let default_input_data = default_fields
                .get(key)
                .expect("Default InputData to exist")
                .clone();
            (
                key.clone(),
                (
                    create_node_ref(cx),
                    error_signal,
                    value_signal,
                    Arc::new(default_input_data),
                ),
            )
        })
        .collect();
    FormSubmitData::new(input_elements, meta_data.clone())
}
