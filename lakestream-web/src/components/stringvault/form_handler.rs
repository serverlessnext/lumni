use std::collections::HashMap;

use async_trait::async_trait;
use leptos::html::Div;
use leptos::*;
use wasm_bindgen_futures::spawn_local;

use super::{FormView, InputData, SecureStringError, StringVault};

#[async_trait(?Send)]
pub trait ConfigManager: Clone {
    fn get_default_config(&self) -> HashMap<String, String>;
    fn default_fields(&self) -> HashMap<String, InputData>;
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

        create_effect(cx, move |_| {
            let vault_clone = vault_clone.clone();
            let id_string = config_manager_clone.id();
            let default_config = config_manager_clone.get_default_config();
            spawn_local(async move {
                match vault_clone.load_secure_configuration(&id_string).await {
                    Ok(new_config) => {
                        log!("loading config: {:?}", new_config);
                        set_loaded_config(Some(new_config));
                    }
                    Err(e) => match e {
                        SecureStringError::PasswordNotFound(_)
                        | SecureStringError::NoLocalStorageData => {
                            // use default if cant load existing
                            log!("Cant load existing configuration: {:?}", e);
                            set_loaded_config(Some(default_config));
                        }
                        _ => {
                            log!("error loading config: {:?}", e);
                            set_load_config_error(Some(e.to_string()));
                        }
                    },
                };
            });
        });

        let vault_clone = self.vault.clone();
        let uuid = self.config_manager.id();
        let config_manager_clone = self.config_manager.clone();
        let default_config = config_manager_clone.default_fields();
        view! { cx,
            <div>
            {move ||
                if let Some(loaded_config) = loaded_config.get() {

                    view! {
                        cx,
                        <div>
                        <FormView
                            vault={vault_clone.clone()}
                            uuid={uuid.clone()}
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
