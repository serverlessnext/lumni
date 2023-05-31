use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use leptos::log;
use leptos::html::Input;
use leptos::*;

use web_sys::HtmlInputElement;


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
    let is_enabled = input_data.input_field.is_enabled();
    let is_secret = input_data.input_field.is_secret();
    let is_password = input_data.input_field.is_password();

    let masked_value = "******";
    let is_hidden = create_rw_signal(cx, is_secret || is_password);

    let initial_value = if is_hidden.get() { masked_value.to_string() } else { value_signal.get() };
    let display_value_signal = create_rw_signal(cx, initial_value);

    let show_hide_checkbox = is_secret && is_enabled && !is_password;

    view! { cx,
        <div class="bg-blue-200 w-full flex-col items-start text-left mb-4">
            <label class="text-left px-2 w-full">{format!("{} ", label)}</label>
            <input
                type=if is_password { "password" } else { "text" }
                prop:value= { display_value_signal }
                on:input=move |ev| {
                    display_value_signal.set(event_target_value(&ev));
                }
                placeholder= move || {
                    let value = value_signal.get();
                    if value.is_empty() {
                        "none".to_string()
                    } else if is_hidden.get() {
                        masked_value.to_string()
                    } else {
                        value
                    }
                }
                class=get_input_class(is_enabled)
                node_ref=input_ref
                disabled=!is_enabled
            />
            { if show_hide_checkbox {
                view! { cx,
                    <div>
                        <input type="checkbox" on:change=move |_| {
                            is_hidden.set(!is_hidden.get());
                            display_value_signal.set(if is_hidden.get() {
                                masked_value.to_string()
                            } else {
                                value_signal.get()
                            });
                        }
                                > "Show password" </input>
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

#[derive(Debug, Clone)]
pub enum InputField {
    Text { is_enabled: bool },
    Secret { is_enabled: bool },
    Password { is_enabled: bool },
}

impl InputField {
    pub fn new_text(is_enabled: bool) -> Self {
        Self::Text { is_enabled }
    }

    pub fn new_secret(is_enabled: bool) -> Self {
        Self::Secret { is_enabled }
    }

    pub fn new_password(is_enabled: bool) -> Self {
        Self::Password { is_enabled }
    }

    pub fn is_enabled(&self) -> bool {
        match self {
            Self::Text { is_enabled } => *is_enabled,
            Self::Secret { is_enabled } => *is_enabled,
            Self::Password { is_enabled } => *is_enabled,
        }
    }

    pub fn is_secret(&self) -> bool {
        matches!(self, Self::Secret { .. })
    }

    pub fn is_password(&self) -> bool {
        matches!(self, Self::Password { .. })
    }
}

impl Default for InputField {
    fn default() -> Self {
        Self::Text { is_enabled: true }
    }
}



#[derive(Clone)]
pub struct InputData {
    pub value: String,
    pub input_field: InputField,
    pub validator: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
}

impl fmt::Debug for InputData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InputData")
            .field("value", &self.value)
            .field("input_field", &self.input_field)
            .field("validator", &self.validator.is_some())
            .finish()
    }
}

impl InputData {
    pub fn new(
        value: String,
        input_field: InputField,
        validator: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
    ) -> Self {
        Self {
            value,
            input_field,
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
        input_field: InputField,
        validate_fn: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
    ) -> Self {
        Self {
            name: name.to_string(),
            input_data: InputData::new(default, input_field, validate_fn),
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
