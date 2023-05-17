use core::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use leptos::ev::SubmitEvent;
use leptos::html::{Div, Input};
use leptos::*;
use wasm_bindgen_futures::spawn_local;
use web_sys::CryptoKey;

use crate::base::state::GlobalState;
use crate::utils::secure_strings::{load_secure_string, save_secure_string};

#[component]
pub fn ConfigurationFormLoader(
    cx: Scope,
    initial_config: Vec<(String, String)>,
) -> impl IntoView {
    let state = use_context::<RwSignal<GlobalState>>(cx)
        .expect("state to have been provided");

    let (crypto_key, _) = create_slice(
        cx,
        state,
        |state| state.crypto_key.clone(),
        |state, key| state.crypto_key = key,
    );

    let (updated_config, set_updated_config) =
        create_signal(cx, initial_config.clone());
    let updated_config_clone = updated_config.clone();
    let crypto_key_clone = crypto_key.clone();

    create_effect(cx, move |_| {
        if let Some(crypto_key) = crypto_key_clone.get() {
            let updated_config_clone = updated_config_clone.clone();
            spawn_local(async move {
                let updated_config =
                    load_config(&crypto_key, &updated_config_clone.get()).await;
                set_updated_config(updated_config);
            });
        }
    });

    let input_elements = create_input_elements(cx, &updated_config.get());

    let on_submit = {
        let crypto_key = crypto_key.clone();
        let input_elements_clone = input_elements.clone();
        move |ev: SubmitEvent| {
            if let Some(key) = crypto_key.get() {
                handle_form_submission(ev, key, &input_elements_clone);
            } else {
                log!("CryptoKey is not defined, form submission aborted.");
            }
        }
    };

    let on_submit = Rc::new(RefCell::new(on_submit));
    let is_defined = move || crypto_key().is_some();
    view! { cx,
        {move || if is_defined() {
            let on_submit = Rc::clone(&on_submit);
            let input_elements_clone = input_elements.clone();
            view! {
                cx,
                <form class="flex flex-wrap w-96" on:submit=move |ev| (&*on_submit.borrow_mut())(ev)>
                    {updated_config.get().iter().map(move |(key, initial_value)| {
                        let input_ref = input_elements_clone.get(key).expect("input ref to exist").clone();
                        create_input_field_view(cx, key, initial_value, input_ref)
                    }).collect::<Vec<_>>()}
                    <button
                        type="submit"
                        class="bg-amber-600 hover:bg-sky-700 px-5 py-3 text-white rounded-lg"
                    >
                        "Save"
                    </button>
                </form>
            }
        } else {
            // empty form
            view! {cx, <form></form>}
        }}
    }
}

async fn load_config(
    crypto_key: &CryptoKey,
    initial_config: &[(String, String)],
) -> Vec<(String, String)> {
    let mut updated_config = initial_config.iter().cloned().collect::<Vec<_>>();
    for (key_name, value) in &mut updated_config {
        if let Ok(saved_value) = load_secure_string(key_name, crypto_key).await
        {
            *value = saved_value;
        }
    }
    updated_config
}

fn create_input_elements(
    cx: Scope,
    updated_config: &Vec<(String, String)>,
) -> Rc<HashMap<String, NodeRef<Input>>> {
    let mut input_elements: HashMap<String, NodeRef<Input>> = HashMap::new();
    for (key, _value) in updated_config {
        input_elements.insert(key.clone(), create_node_ref(cx));
    }
    Rc::new(input_elements)
}

fn handle_form_submission(
    ev: SubmitEvent,
    crypto_key: CryptoKey,
    input_elements: &Rc<HashMap<String, NodeRef<Input>>>,
) {
    ev.prevent_default();

    for (key, input_ref) in &**input_elements {
        let value = input_ref().expect("input to exist").value();

        let crypto_key = crypto_key.clone();
        let key = key.clone();
        log!("Saving: {} = {}", key, value);
        spawn_local(async move {
            match save_secure_string(&key, &value, &crypto_key).await {
                Ok(_) => {
                    log!("Successfully saved secure data for key: {}", key);
                }
                Err(e) => {
                    log!(
                        "Failed to save secure data for key: {}. Error: {:?}",
                        key,
                        e
                    );
                }
            };
        });
    }
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
