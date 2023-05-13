use std::collections::HashMap;

use leptos::*;
use leptos::ev::SubmitEvent;
use leptos::html::Input;

use crate::utils::local_storage::{save_data, load_data};


#[component]
pub fn ConfigurationForm(cx: Scope, initial_config: Vec<(String, String)>) -> impl IntoView {

    let mut input_elements: HashMap<String, NodeRef<Input>> = HashMap::new();

    // Overwrite initial_config with saved data if available
    let updated_config: Vec<(String, String)> = initial_config.iter().map(|(key, value)| {
        if let Some(saved_value) = load_data(key) {
            log!("Loaded: {} = {}", key, saved_value);
            (key.clone(), saved_value)
        } else {
            (key.clone(), value.clone())
        }
    }).collect();
    for (key, _value) in &updated_config {
        input_elements.insert(key.clone(), create_node_ref(cx));
    }

    let input_elements_clone = input_elements.clone();

    let on_submit = move |ev: SubmitEvent| {
        ev.prevent_default();

        let mut config_hashmap = HashMap::new();

        for (key, input_ref) in &input_elements_clone {
            let value = input_ref()
                .expect("input to exist")
                .value();
            save_data(key, &value);
            config_hashmap.insert(key.clone(), value);
        }

        log!("Saved: {:?}", config_hashmap);
    };

    view! { cx,
        <form class="flex flex-wrap w-96" on:submit=on_submit>
            {updated_config.iter().map(move |(key, initial_value)| {
                let input_ref = input_elements.get(key).expect("input ref to exist").clone();
                view! {
                    cx,
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
            }).collect::<Vec<_>>()}
            <button
                type="submit"
                class="bg-amber-600 hover:bg-sky-700 px-5 py-3 text-white rounded-lg"
            >
                "Save"
            </button>
        </form>
    }

}
