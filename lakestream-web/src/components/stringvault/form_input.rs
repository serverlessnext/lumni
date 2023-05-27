use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use leptos::html::Input;
use leptos::*;

type InputElement = (
    NodeRef<Input>,
    RwSignal<Option<String>>,
    RwSignal<String>,
    Arc<InputData>,
);
pub type InputElements = HashMap<String, InputElement>;

#[component]
pub fn InputFieldView(
    cx: Scope,
    label: String,
    input_element: InputElement,
) -> impl IntoView {
    let (input_ref, error_signal, value_signal, input_data) = input_element;
    let (is_hidden, set_is_hidden) =
        create_signal(cx, input_data.opts.is_secret);
    let masked_value = "******";

    let is_secret = input_data.opts.is_secret;
    let is_enabled = input_data.opts.is_enabled;

    view! { cx,
        <div class="bg-blue-200 w-full flex-col items-start text-left mb-4">
            <label class="text-left px-2 w-full">{format!("{} ", label)}</label>
            <input
                type="text"
                value= move || if is_hidden.get() { masked_value.to_string() } else { value_signal.get() }
                class=get_input_class(is_enabled)
                node_ref=input_ref
                disabled=!is_enabled
            />
            { if is_secret && is_enabled {
                view! { cx,
                    <div>
                    <input type="checkbox" on:change=move |_| set_is_hidden(!is_hidden.get())> "Show password" </input>
                    </div>
                }
            } else {
                view! { cx, <div></div> }
            } }
            <div class="text-red-500">
                { move || error_signal.get().unwrap_or("".to_string()) }
            </div>
        </div>
    }
}

fn get_input_class(is_enabled: bool) -> &'static str {
    if is_enabled {
        "shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 \
         leading-tight focus:outline-none focus:shadow-outline"
    } else {
        "shadow appearance-none border rounded w-full py-2 px-3 text-gray-300 \
         leading-tight focus:outline-none focus:shadow-outline"
    }
}

#[derive(Clone, Debug)]
pub struct InputElementOpts {
    pub is_secret: bool,
    pub is_enabled: bool,
}

impl Default for InputElementOpts {
    fn default() -> Self {
        Self {
            is_secret: false,
            is_enabled: true,
        }
    }
}

impl InputElementOpts {
    pub fn new(is_secret: bool, is_enabled: bool) -> Self {
        Self {
            is_secret,
            is_enabled,
        }
    }
}

#[derive(Clone)]
pub struct InputData {
    pub value: String,
    pub opts: InputElementOpts,
    pub validator: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
}

impl fmt::Debug for InputData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InputData")
            .field("value", &self.value)
            .field("opts", &self.opts)
            .field("validator", &self.validator.is_some())
            .finish()
    }
}

impl InputData {
    pub fn new(
        value: String,
        opts: InputElementOpts,
        validator: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
    ) -> Self {
        Self {
            value,
            opts,
            validator,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FormInputField {
    pub name: String,
    pub input_data: InputData,
}

impl FormInputField {
    pub fn new(
        name: &str,
        default: String,
        opts: InputElementOpts,
        validate_fn: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
    ) -> Self {
        Self {
            name: name.to_string(),
            input_data: InputData::new(default, opts, validate_fn),
        }
    }

    pub fn to_input_data(self) -> (String, InputData) {
        (self.name, self.input_data)
    }
}

pub fn create_input_elements(
    cx: Scope,
    updated_config: &HashMap<String, String>,
    default_config: &HashMap<String, InputData>,
) -> InputElements {
    updated_config
        .iter()
        .map(|(key, value)| {
            let error_signal = create_rw_signal(cx, None);
            let value_signal = create_rw_signal(cx, value.clone());
            let default_input_data = default_config
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
        .collect()
}
