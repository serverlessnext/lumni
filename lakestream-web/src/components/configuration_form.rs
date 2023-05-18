use core::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use leptos::ev::SubmitEvent;
use leptos::html::{Div, Input};
use leptos::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::CryptoKey;

use crate::base::GlobalState;
use crate::components::login_form::LoginForm;
use crate::utils::secure_strings::{
    load_secure_configuration, save_secure_configuration,
};
use crate::LakestreamError;

#[component]
pub fn ObjectStoreConfig(
    cx: Scope,
    uuid: String,
    initial_config: HashMap<String, String>,
) -> impl IntoView {
    let state = use_context::<RwSignal<GlobalState>>(cx)
        .expect("state to have been provided");

    let (crypto_key, set_crypto_key) = create_slice(
        cx,
        state,
        |state| state.crypto_key.clone(),
        |state, key| state.crypto_key = key,
    );

    let (loaded_config, set_loaded_config) = create_signal(cx, None);
    let (load_config_error, set_load_config_error) =
        create_signal(cx, None::<String>);

    let (is_override_active, set_is_override_active) = create_signal(cx, false);

    // OVERRIDE INTERNAL DECRYPT ERROR - REMOVE WHEN RESET IS ADDED TO FORM
    // this will show the form with empty values, so the user has an option to fill in new
    // next TODO is to add a reset button to the form to clear existing values/ set new password
    set_is_override_active(true);
    let initial_config_reset = initial_config.clone();

    let uuid_clone = uuid.clone();
    let is_override = is_override_active.get();
    create_effect(cx, move |_| {
        if let Some(crypto) = crypto_key.get() {
            let uuid_clone = uuid_clone.clone();
            spawn_local(async move {
                match load_config(&crypto, &uuid_clone).await {
                    Ok(new_config) => {
                        set_loaded_config(Some(new_config));
                        set_load_config_error(None); // Clear the error if loading was successful
                    }
                    Err(e) => {
                        set_load_config_error(Some(e.to_string()));
                        // Skip the reset of crypto_key when override is active
                        if !is_override {
                            // Reset the crypto_key to prompt the user again via LoginForm
                            set_crypto_key(None);
                        }
                    }
                };
            });
        }
    });

    view! { cx,
        {move ||
            if let Some(crypto_key) = crypto_key.get() {
                if let Some(loaded_config) = loaded_config.get() {
                    form_view(cx, crypto_key, uuid.clone(), &loaded_config)
                } else if is_override_active.get() {
                    form_view(cx, crypto_key, uuid.clone(), &initial_config_reset)
                } else {
                    view! {
                        cx,
                        <div>
                            "Loading..."
                        </div>
                    }
                }
            } else if let Some(error) = load_config_error.get() {
                view! {
                    cx,
                    <div>
                        {"Error loading configuration: "}
                        {error}
                        <LoginForm/>
                    </div>
                }
            } else {
                view! {
                    cx,
                    <div>
                    <LoginForm/>
                    </div>
                }
            }
        }
    }
}

pub trait OnSubmit {
    fn call(
        &mut self,
        ev: SubmitEvent,
        input_elements: HashMap<String, NodeRef<Input>>,
    );
}

impl<F: FnMut(SubmitEvent, HashMap<String, NodeRef<Input>>)> OnSubmit for F {
    fn call(
        &mut self,
        ev: SubmitEvent,
        input_elements: HashMap<String, NodeRef<Input>>,
    ) {
        self(ev, input_elements)
    }
}

fn form_view(
    cx: Scope,
    crypto_key: CryptoKey,
    uuid: String,
    loaded_config: &HashMap<String, String>,
) -> HtmlElement<Div> {
    let input_elements = create_input_elements(cx, loaded_config);
    let input_elements_clone_submit = input_elements.clone();
    let uuid_clone = uuid.clone();

    let on_submit: Rc<RefCell<dyn OnSubmit>> = Rc::new(RefCell::new(
        move |ev: SubmitEvent,
              input_elements: HashMap<String, NodeRef<Input>>| {
            let crypto_key = crypto_key.clone();
            handle_form_submission(ev, crypto_key, &uuid_clone, input_elements);
        },
    ));

    view! {
        cx,
        <div>
        <form class="flex flex-wrap w-96" on:submit=move |ev| {on_submit.borrow_mut().call(ev, input_elements_clone_submit.clone())}>
            {loaded_config.iter().map(move |(key, initial_value)| {
                let input_elements_clone_map = input_elements.clone();
                let input_ref = input_elements_clone_map.get(key).expect("input ref to exist").clone();
                create_input_field_view(cx, key, initial_value, input_ref)
            }).collect::<Vec<_>>()}
            <button
                type="submit"
                class="bg-amber-600 hover:bg-sky-700 px-5 py-3 text-white rounded-lg"
            >
                "Save"
            </button>
        </form>
        </div>
    }
}

fn create_input_elements(
    cx: Scope,
    updated_config: &HashMap<String, String>,
) -> HashMap<String, NodeRef<Input>> {
    let mut input_elements: HashMap<String, NodeRef<Input>> = HashMap::new();
    for (key, _value) in updated_config {
        input_elements.insert(key.clone(), create_node_ref(cx));
    }
    input_elements
}

async fn load_config(
    crypto_key: &CryptoKey,
    uuid: &str,
) -> Result<HashMap<String, String>, LakestreamError> {
    log!("Loading config for uuid: {}", uuid);

    // Here we directly use load_secure_configuration() to get the entire config
    let config = load_secure_configuration(uuid, crypto_key).await?;

    Ok(config)
}

fn handle_form_submission(
    ev: SubmitEvent,
    crypto_key: CryptoKey,
    uuid: &str,
    input_elements: HashMap<String, NodeRef<Input>>,
) {
    ev.prevent_default();

    log!("Saving items for uuid: {}", uuid);

    let mut config: HashMap<String, String> = HashMap::new();
    for (key, input_ref) in &input_elements {
        let value = input_ref().expect("input to exist").value();
        config.insert(key.clone(), value);
    }

    let crypto_key = crypto_key.clone();
    let uuid = uuid.to_string();
    spawn_local(async move {
        match save_secure_configuration(&uuid, config, &crypto_key).await {
            Ok(_) => {
                log!(
                    "Successfully saved secure configuration for uuid: {}",
                    uuid
                );
            }
            Err(e) => {
                log!(
                    "Failed to save secure configuration for uuid: {}. Error: \
                     {:?}",
                    uuid,
                    e
                );
            }
        };
    });
    log!("Saved items");
}

fn create_input_field_view(
    cx: Scope,
    key: &String,
    initial_value: &String,
    input_ref: NodeRef<Input>,
) -> HtmlElement<Div> {
    view! { cx,
        <div class="bg-blue-200 w-full flex-col items-start text-left mb-4">
            <label class="text-left px-2 w-full">{format!("{} ", key)}</label>
            <input
                type="text"
                value=initial_value
                class="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
                node_ref=input_ref
            />
        </div>
    }
}
