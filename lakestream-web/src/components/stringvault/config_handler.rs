use std::collections::HashMap;

use async_trait::async_trait;
use leptos::ev::SubmitEvent;
use leptos::html::{Div, Input};
use leptos::*;
use wasm_bindgen_futures::spawn_local;

use super::{InputData, SecureStringError, StringVault};

type InputElement =
    (NodeRef<Input>, RwSignal<Option<String>>, RwSignal<String>);
type InputElements = HashMap<String, InputElement>;

#[async_trait(?Send)]
pub trait ConfigManager: Clone {
    fn get_default_config(&self) -> HashMap<String, String>;
    fn default_fields(&self) -> HashMap<String, InputData>;
    fn id(&self) -> String;
}

pub struct ConfigFormView<T: ConfigManager + Clone + 'static> {
    config_manager: T,
    vault: StringVault,
}

impl<T: ConfigManager + Clone + 'static> ConfigFormView<T> {
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

#[component]
fn FormView(
    cx: Scope,
    vault: StringVault,
    uuid: String,
    initial_config: HashMap<String, String>,
    default_config: HashMap<String, InputData>,
) -> impl IntoView {
    let (is_submitting, set_is_submitting) = create_signal(cx, false);
    let (submit_error, set_submit_error) = create_signal(cx, None::<String>);

    let input_elements = create_input_elements(cx, &initial_config);
    let input_elements_clone_submit = input_elements.clone();

    let on_submit = {
        move |ev: SubmitEvent, input_elements: InputElements| {
            ev.prevent_default(); // prevent form submission

            // Validate input elements
            let mut validation_errors = HashMap::new();

            for (key, (input_ref, _, _)) in &input_elements {
                let value = input_ref().expect("input to exist").value();
                let validator = default_config
                    .get(key)
                    .expect("Validator to exist")
                    .validator
                    .clone();

                if let Err(e) = validator(&value) {
                    log::error!("Validation failed: {}", e);
                    validation_errors.insert(key.clone(), e.to_string());
                    ev.prevent_default(); // prevent form submission
                }
            }

            // Write validation errors to corresponding WriteSignals
            for (key, (_, error_signal, _)) in &input_elements {
                if let Some(error) = validation_errors.get(key) {
                    error_signal.set(Some(error.clone()));
                } else {
                    error_signal.set(None);
                }
            }

            // If there are no validation errors, handle form submission
            if validation_errors.is_empty() {
                log!("Validation successful");
                handle_form_submission(
                    vault.clone(),
                    uuid.clone(),
                    input_elements,
                    set_is_submitting,
                    set_submit_error,
                );
            }
        }
    };

    view! {
        cx,
        <div>
            <form class="flex flex-wrap w-96"
                  on:submit=move |ev| {
                    set_is_submitting(true);
                    on_submit(ev, input_elements_clone_submit.clone())
                  }
            >
            <For
                each= move || {input_elements.clone().into_iter().enumerate()}
                    key=|(index, _input)| *index
                    view= move |cx, (_, (key, (input_ref, error_signal, value_signal)))| {
                        view! {
                            cx,
                            <CreateInputFieldView
                                key={key}
                                input_ref={input_ref}
                                error_signal={error_signal}
                                value_signal={value_signal}
                            />
                        }
                }
            />
            <button
                type="submit"
                class="bg-amber-600 hover:bg-sky-700 px-5 py-3 text-white rounded-lg"
            >
                "Save"
            </button>
            </form>

        // Show a loading message while the form is submitting
        { move || if is_submitting.get() {
            view! {
                cx,
                <div>
                    "Submitting..."
                </div>
            }
        } else {
            view! {
                cx,
                <div></div>
            }
        }}

        // Show an error message if there was an error during submission
        { move || if let Some(error) = submit_error.get() {
            view! {
                cx,
                <div class="text-red-500">
                    {"Error during submission: "}
                    {error}
                </div>
            }
        } else {
            view! {
                cx,
                <div></div>
            }
        }}
        </div>
    }
}

#[component]
fn CreateInputFieldView(
    cx: Scope,
    key: String,
    input_ref: NodeRef<Input>,
    error_signal: RwSignal<Option<String>>,
    value_signal: RwSignal<String>,
) -> impl IntoView {
    view! { cx,
        <div class="bg-blue-200 w-full flex-col items-start text-left mb-4">
            <label class="text-left px-2 w-full">{format!("{} ", key)}</label>
            <input
                type="text"
                value=value_signal.get()
                class="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
                node_ref=input_ref
            />
            <div class="text-red-500">
                { move || match error_signal.get() {
                    Some(error) => error.clone(),
                    None => "".to_string(),
                }}
            </div>
        </div>
    }
}

pub trait OnSubmit {
    fn call(&mut self, ev: SubmitEvent, input_elements: InputElements);
}

impl<F: FnMut(SubmitEvent, InputElements)> OnSubmit for F {
    fn call(&mut self, ev: SubmitEvent, input_elements: InputElements) {
        self(ev, input_elements)
    }
}

fn create_input_elements(
    cx: Scope,
    updated_config: &HashMap<String, String>,
) -> InputElements {
    let mut input_elements: InputElements = HashMap::new();
    for (key, value) in updated_config {
        let error_signal = create_rw_signal(cx, None);
        let value_signal = create_rw_signal(cx, value.clone());
        input_elements.insert(
            key.clone(),
            (create_node_ref(cx), error_signal, value_signal),
        );
    }
    input_elements
}

fn handle_form_submission(
    mut vault: StringVault,
    uuid: String,
    input_elements: InputElements,
    set_is_submitting: WriteSignal<bool>,
    set_submit_error: WriteSignal<Option<String>>,
) {
    let config = extract_config(&input_elements);
    spawn_local(async move {
        match vault.save_secure_configuration(&uuid, config.clone()).await {
            Ok(_) => {
                log!("Successfully saved secure configuration: {:?}", uuid);
                for (key, value) in &config {
                    if let Some((_, _, value_signal)) = input_elements.get(key)
                    {
                        value_signal.set(value.clone());
                    }
                }
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

fn extract_config(input_elements: &InputElements) -> HashMap<String, String> {
    let mut config: HashMap<String, String> = HashMap::new();
    for (key, (input_ref, _, value_writer)) in input_elements {
        let value = input_ref().expect("input to exist").value();
        config.insert(key.clone(), value.clone());
        value_writer.set(value);
    }
    config
}

fn save_config(
    config: HashMap<String, String>,
    mut vault: StringVault,
    uuid: String,
    input_elements: InputElements,
) {
    spawn_local(async move {
        //let mut vault = vault.borrow_mut();
        match vault.save_secure_configuration(&uuid, config.clone()).await {
            Ok(_) => {
                log!("Successfully saved secure configuration: {:?}", uuid);
                for (key, value) in &config {
                    if let Some((_, _, value_writer)) = input_elements.get(key)
                    {
                        value_writer.set(value.clone());
                    }
                }
            }
            Err(e) => {
                log!("Failed to save secure configuration. Error: {:?}", e);
            }
        };
    });
    log!("Saved items");
}
