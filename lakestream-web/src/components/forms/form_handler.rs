use std::collections::HashMap;

use leptos::html::Div;
use leptos::*;
use wasm_bindgen_futures::spawn_local;

use crate::stringvault::{StringVault, ObjectKey, SecureStringError};

pub use super::form_input::InputData;
pub use super::form_input_builder::FormInputFieldBuilder;
use super::form_view::FormView;


#[derive(Clone, PartialEq, Debug)]
pub struct FormOwner {
    pub tag: String,
    pub id: String,
}

impl FormOwner {
    pub fn new_with_form_tag(id: String) -> Self {
        Self {
            tag: "FORM".to_string(),
            id,
        }
    }

    pub fn to_object_key(&self) -> ObjectKey {
        ObjectKey {
            tag: self.tag.clone(),
            id: self.id.clone(),
        }
    }
}

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

    fn form_owner(&self) -> FormOwner {
        FormOwner::new_with_form_tag(self.config_manager.id())
    }

    pub fn form_data_handler(&self, cx: Scope) -> HtmlElement<Div> {
        let (loaded_config, set_loaded_config) = create_signal(cx, None);
        let (load_config_error, set_load_config_error) =
            create_signal(cx, None::<String>);

        let vault_clone = self.vault.clone();
        let config_manager_clone = self.config_manager.clone();
        let form_owner_clone = self.form_owner();

        create_effect(cx, move |_| {
            let vault_clone = vault_clone.clone();
            let default_config = config_manager_clone
                .default_fields()
                .into_iter()
                .map(|(key, input_data)| (key, input_data.value))
                .collect();

            let form_owner = form_owner_clone.clone();
            spawn_local(async move {
                match vault_clone.load_secure_configuration(form_owner.to_object_key()).await {
                    Ok(new_config) => {
                        set_loaded_config(Some(new_config));
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
        let form_owner_clone = self.form_owner();
        view! { cx,
            <div>
            {move ||
                if let Some(loaded_config) = loaded_config.get() {

                    view! {
                        cx,
                        <div>
                        <FormView
                            vault={vault_clone.clone()}
                            form_owner={form_owner_clone.clone()}
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
    form_owner: FormOwner,
    form_config: HashMap<String, String>,
    set_is_submitting: WriteSignal<bool>,
    set_submit_error: WriteSignal<Option<String>>,
) {
    let form_id = form_owner.id.clone();
    spawn_local(async move {
        match vault
            .save_secure_configuration(form_owner.to_object_key(), form_config)
            .await
        {
            Ok(_) => {
                log!("Successfully saved secure configuration: {:?}", form_id);
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
