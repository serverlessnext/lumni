use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use leptos::html::Input;
use leptos::*;

use crate::components::icons::LockIconView;

type InputElement = (
    NodeRef<Input>,
    RwSignal<Option<String>>,
    RwSignal<String>,
    Arc<InputData>,
);
pub type InputElements = HashMap<String, InputElement>;

const MASKED_VALUE: &str = "*****";

#[component]
pub fn InputBoxView(
    cx: Scope,
    label_text: String,
    input_element: InputElement,
    input_changed: RwSignal<bool>,
) -> impl IntoView {
    // shows Label, InputField and Error
    // defined from input_element
    let (input_ref, error_signal, value_signal, input_data) = input_element;
    let is_secret = input_data.input_field.is_secret();
    let is_password = input_data.input_field.is_password();
    let initial_enabled = input_data.input_field.is_enabled();

    // show lock icon if secret and not password (passwords cant be unlocked)
    let show_lock_icon = is_secret && initial_enabled && !is_password;

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
    let is_enabled = (move || {
        if is_locked.get() {
            false
        } else {
            initial_enabled
        }
    })
    .derive_signal(cx);

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

    let click_handler: Box<dyn Fn()> = Box::new(move || {
        let new_state = !is_locked.get();
        let current_value = value_signal.get();
        is_locked.set(new_state);
        display_value_signal.set(if new_state {
            MASKED_VALUE.to_string()
        } else {
            current_value
        });
    });

    let icon_view: View = if show_lock_icon {
        view! {
            cx,
            <div class="w-8">
                <LockIconView
                    is_locked
                    click_handler
                />
            </div>
        }
        .into_view(cx)
    } else {
        view! { cx, }.into_view(cx)
    };

    view! {
        cx,
        <div class="w-full flex-col items-start text-left mb-2 p-2 bg-white text-gray-800">
            <InputFieldLabelView
                label_text
                icon_view=icon_view
            />
            <InputFieldView
                input_ref
                is_password
                is_enabled
                value_signal
                display_value_signal
                input_changed
            />
            <InputFieldErrorView error_signal/>
        </div>
    }
}

#[component]
pub fn InputFieldLabelView(
    cx: Scope,
    label_text: String,
    icon_view: View,
) -> impl IntoView {
    view! {
        cx,
        <div class="flex justify-between items-center">
            <label for="field_id" class="text-base font-semibold text-gray-900">{label_text}</label>
            {icon_view}
        </div>

    }
}

#[component]
pub fn InputFieldView(
    cx: Scope,
    input_ref: NodeRef<Input>,
    is_password: bool,
    is_enabled: Signal<bool>,
    value_signal: RwSignal<String>,
    display_value_signal: RwSignal<String>,
    input_changed: RwSignal<bool>,
) -> impl IntoView {
    view! { cx,
        <input
            type=if is_password { "password" } else { "text" }
            prop:value= { display_value_signal }
            on:input=move |ev| {
                if is_enabled.get() {
                    let value = event_target_value(&ev);
                    value_signal.set(value);
                    input_changed.set(true);    // enable submit button
                }
            }
            placeholder="none".to_string()
            class=move || {format!("{} w-full", get_input_class(is_enabled.get()))}
            node_ref=input_ref
            disabled=move || { !is_enabled.get() }
        />
    }
}

fn get_input_class(is_enabled: bool) -> &'static str {
    if is_enabled {
        "bg-gray-50 border border-gray-300 text-gray-900 rounded-lg \
         focus:ring-blue-500 focus:border-blue-500 block w-full p-2.5"
    } else {
        "bg-gray-50 border border-gray-300 text-gray-900 rounded-lg block \
         w-full p-2.5"
    }
}

#[component]
pub fn InputFieldErrorView(
    cx: Scope,
    error_signal: RwSignal<Option<String>>,
) -> impl IntoView {
    view! { cx,
        <div class="text-red-500">
            { move || error_signal.get().unwrap_or("".to_string()) }
        </div>
    }
    .into_view(cx)
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
