use std::collections::HashMap;

use leptos::html::Div;
use leptos::*;
use localencrypt::{ItemMetaData, LocalEncrypt, SecureStringError};
use serde_json;
use wasm_bindgen_futures::spawn_local;

pub use super::form_input::{InputData, InputField};
pub use super::form_input_builder::{FormInputFieldBuilder, InputFieldPattern};
use super::form_view::FormView;

pub trait ConfigManager: Clone {
    fn default_fields(&self) -> HashMap<String, InputData>;
    fn name(&self) -> String;
    fn id(&self) -> String;
}

pub struct FormHandler<T: ConfigManager + Clone + 'static> {
    config_manager: T,
    vault: LocalEncrypt,
}

impl<T: ConfigManager + Clone + 'static> FormHandler<T> {
    pub fn new(config_manager: T, vault: LocalEncrypt) -> Self {
        Self {
            config_manager,
            vault,
        }
    }

    pub fn form_data_handler(&self, cx: Scope) -> HtmlElement<Div> {
        let (loaded_config, set_loaded_config) = create_signal(cx, None);
        let (load_config_error, set_load_config_error) =
            create_signal(cx, None::<String>);

        let config_manager_clone = self.config_manager.clone();
        let form_name = self.config_manager.id();

        let local_storage = match self.vault.backend() {
            localencrypt::StorageBackend::Browser(browser_storage) => {
                browser_storage.local_storage().unwrap_or_else(|| panic!("Invalid browser storage type"))
            },
            _ => panic!("Invalid storage backend"),
        };

        create_effect(cx, move |_| {
            let default_config = config_manager_clone
                .default_fields()
                .into_iter()
                .map(|(key, input_data)| (key, input_data.value))
                .collect();

            let local_storage = local_storage.clone();
            let form_name_clone = form_name.clone();
            spawn_local(async move {
                match local_storage.load_content(&form_name_clone).await {
                    Ok(Some(data)) => match serde_json::from_slice(&data) {
                        Ok(new_config) => {
                            set_loaded_config(Some(new_config));
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
                            &form_name_clone
                        );
                        set_loaded_config(Some(default_config));
                    }
                    Err(e) => {
                        match e {
                            SecureStringError::PasswordNotFound(_)
                            | SecureStringError::NoLocalStorageData => {
                                // use default if cant load existing
                                log::info!(
                                    "Cant load existing configuration. \
                                     Creating new."
                                );
                                set_loaded_config(Some(default_config));
                            }
                            _ => {
                                log::error!("error loading config: {:?}", e);
                                set_load_config_error(Some(e.to_string()));
                            }
                        }
                    }
                }
            });
        });

        let vault_clone = self.vault.clone();
        let config_manager_clone = self.config_manager.clone();
        let default_config = config_manager_clone.default_fields();

        let mut tags = HashMap::new();
        tags.insert("Name".to_string(), self.config_manager.name());
        let meta_data =
            ItemMetaData::new_with_tags(&self.config_manager.id(), tags);

        view! { cx,
            <div>
            {move ||
                if let Some(loaded_config) = loaded_config.get() {

                    view! {
                        cx,
                        <div>
                        <FormView
                            vault={vault_clone.clone()}
                            meta_data={meta_data.clone()}
                            initial_config={loaded_config}
                            default_config={default_config.clone()}
                        />
                        </div>
                    }
                }
                else if let Some(error) = load_config_error.get() {
                    view! {
                        cx,
                        <div>
                            {"Error loading configuration: "}
                            {error}
                        </div>
                    }
                }
                else {
                    view! {
                        cx,
                        <div>
                            "Loading..."
                        </div>
                    }
                }
            }
            </div>
        }
    }
}

pub fn handle_form_submission(
    vault: LocalEncrypt,
    meta_data: ItemMetaData,
    document_content: Vec<u8>,
    set_is_submitting: WriteSignal<bool>,
    set_submit_error: WriteSignal<Option<String>>,
) {

    let mut local_storage = match vault.backend() {
        localencrypt::StorageBackend::Browser(browser_storage) => {
            browser_storage.local_storage().unwrap_or_else(|| panic!("Invalid browser storage type"))
        },
        _ => panic!("Invalid storage backend"),
    };

    spawn_local(async move {
        match local_storage.save_content(meta_data, &document_content).await {
            Ok(_) => {
                log!("Successfully saved secure configuration",);
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
