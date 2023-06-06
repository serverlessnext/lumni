use std::collections::HashMap;
use serde_json;

use leptos::html::Div;
use leptos::*;
use stringvault::{FormMetaData, SecureStringError, StringVault};
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
    vault: StringVault,
}

impl<T: ConfigManager + Clone + 'static> FormHandler<T> {
    pub fn new(config_manager: T, vault: StringVault) -> Self {
        Self {
            config_manager,
            vault,
        }
    }

    pub fn form_data_handler(&self, cx: Scope) -> HtmlElement<Div> {
        let (loaded_config, set_loaded_config) = create_signal(cx, None);
        let (load_config_error, set_load_config_error) =
            create_signal(cx, None::<String>);

        let vault_clone = self.vault.clone();
        let config_manager_clone = self.config_manager.clone();
        let form_name = self.config_manager.id();

        create_effect(cx, move |_| {
            let vault_clone = vault_clone.clone();
            let default_config = config_manager_clone
                .default_fields()
                .into_iter()
                .map(|(key, input_data)| (key, input_data.value))
                .collect();

            let form_name_clone = form_name.clone();
            spawn_local(async move {
                match vault_clone.load_configuration(&form_name_clone).await {
                    Ok(data) => {
                        match serde_json::from_slice(&data) {
                            Ok(new_config) => {
                                set_loaded_config(Some(new_config));
                            }
                            Err(e) => {
                                log::error!("error deserializing config: {:?}", e);
                                set_load_config_error(Some(e.to_string()));
                            }
                        }
                    }
                    Err(e) => match e {
                        SecureStringError::PasswordNotFound(_)
                        | SecureStringError::NoLocalStorageData => {
                            // use default if cant load existing
                            log::info!(
                                "Cant load existing configuration. Creating \
                                 new."
                            );
                            set_loaded_config(Some(default_config));
                        }
                        _ => {
                            log::error!("error loading config: {:?}", e);
                            set_load_config_error(Some(e.to_string()));
                        }
                    },
                };
            });
        });

        let vault_clone = self.vault.clone();
        let config_manager_clone = self.config_manager.clone();
        let default_config = config_manager_clone.default_fields();
        let form_meta = FormMetaData::new(
            self.config_manager.id(),
            self.config_manager.name(),
        );

        view! { cx,
            <div>
            {move ||
                if let Some(loaded_config) = loaded_config.get() {

                    view! {
                        cx,
                        <div>
                        <FormView
                            vault={vault_clone.clone()}
                            form_meta={form_meta.clone()}
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
    mut vault: StringVault,
    form_meta: FormMetaData,
    form_data: Vec<u8>,
    set_is_submitting: WriteSignal<bool>,
    set_submit_error: WriteSignal<Option<String>>,
) {
    spawn_local(async move {
        match vault
            .save_configuration(form_meta, &form_data)
            .await
        {
            Ok(_) => {
                log!(
                    "Successfully saved secure configuration",
                );
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
