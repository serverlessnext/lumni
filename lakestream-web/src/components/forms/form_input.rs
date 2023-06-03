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

const MASKED_VALUE: &str = "*****";

#[component]
pub fn LockIcon(cx: Scope, is_locked: bool) -> impl IntoView {
    if is_locked {
        view! {
            cx,
            <svg xmlns="http://www.w3.org/2000/svg" fill="white" viewBox="0 0 32 32" stroke-width="1" stroke="orange" class="w-10 h-10 py-1">
                <path stroke-linecap="round" stroke-linejoin="round" d="M16.5 10.5V6.75a4.5 4.5 0 10-9 0v3.75m-.75 11.25h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H6.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z" />
            </svg>
        }
    } else {
        view! {
            cx,
            <svg xmlns="http://www.w3.org/2000/svg" fill="white" viewBox="0 0 32 32" stroke-width="1" stroke="green" class="w-10 h-10 py-1">
                <path stroke-linecap="round" stroke-linejoin="round" d="M13.5 10.5V6.75a4.5 4.5 0 119 0v3.75M3.75 21.75h10.5a2.25 2.25 0 002.25-2.25v-6.75a2.25 2.25 0 00-2.25-2.25H3.75a2.25 2.25 0 00-2.25 2.25v6.75a2.25 2.25 0 002.25 2.25z" />
            </svg>
        }
    }
}

#[component]
pub fn LockIconView(
    cx: Scope,
    is_locked: RwSignal<bool>,
    display_value_signal: RwSignal<String>,
    value_signal: RwSignal<String>,
) -> impl IntoView {
    let is_value_empty = move || value_signal.get().is_empty();

    view! { cx,
        <div
            on:click=move |_| {
                let new_state = !is_locked.get();
                let current_value = value_signal.get();
                is_locked.set(new_state);
                display_value_signal.set(if new_state {
                    MASKED_VALUE.to_string()
                } else {
                    current_value
                });
            }
            disabled=is_value_empty
        >
            {move || if is_locked.get() {
                view! {cx, <LockIcon is_locked=true /> }
            } else {
                view! {cx, <LockIcon is_locked=false /> }
            }}
        </div>
    }
}

#[component]
pub fn InputFieldView(
    cx: Scope,
    label: String,
    input_element: InputElement,
    input_changed: RwSignal<bool>,
) -> impl IntoView {
    // defined from input_element
    let (input_ref, error_signal, value_signal, input_data) = input_element;
    let is_enabled = input_data.input_field.is_enabled();
    let is_secret = input_data.input_field.is_secret();
    let is_password = input_data.input_field.is_password();
    let show_edit_checkbox = is_secret && is_enabled && !is_password;

    // signals
    let initial_value = value_signal.get();
    let is_locked = create_rw_signal(
        cx,
        if initial_value.is_empty() {
            false
        } else {
            is_secret || is_password
        },
    );
    let initial_value = if is_locked.get() {
        if initial_value.is_empty() {
            "".to_string()
        } else {
            MASKED_VALUE.to_string()
        }
    } else {
        initial_value
    };
    let display_value_signal = create_rw_signal(cx, initial_value);

    view! { cx,
        <div class="bg-blue-200 w-full flex-col items-start text-left mb-4">
            <label class="text-left px-2 w-full">{format!("{} ", label)}</label>
            <div class="flex items-center">
                { if show_edit_checkbox {
                    view! { cx, <div class="w-8"><LockIconView is_locked display_value_signal value_signal /></div> }
                }
                else {
                    view! { cx, <div class="w-8"></div> }
                }}
                <input
                    type=if is_password { "password" } else { "text" }
                    prop:value= { display_value_signal }
                    on:input=move |ev| {
                        if !is_locked.get() {
                            let value = event_target_value(&ev);
                            value_signal.set(value.clone());
                            display_value_signal.set(value);
                            input_changed.set(true);
                    }}
                    placeholder= move || {
                        let value = value_signal.get();
                        if value.is_empty() {
                            "none".to_string()
                        } else if is_locked.get() {
                            MASKED_VALUE.to_string()
                        } else {
                            value
                        }
                    }
                    class=get_input_class(is_enabled)
                    node_ref=input_ref
                    disabled=move || {
                        if is_locked.get() {
                            true
                        } else {
                            !is_enabled
                        }
                    }
                />
            </div>
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

#[component]
pub fn CheckboxView(
    cx: Scope,
    is_locked: RwSignal<bool>,
    display_value_signal: RwSignal<String>,
    value_signal: RwSignal<String>,
) -> impl IntoView {
    let is_value_empty = move || value_signal.get().is_empty();

    view! { cx,
        <input type="checkbox"
            on:change=move |_| {
                is_locked.set(!is_locked.get());
                display_value_signal.set(if is_locked.get() {
                    MASKED_VALUE.to_string()
                } else {
                    value_signal.get()
                });
            }
            checked=!is_locked.get()
            disabled=is_value_empty
        > "Edit" </input>
    }
}
