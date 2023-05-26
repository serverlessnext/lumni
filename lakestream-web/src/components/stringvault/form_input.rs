use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use leptos::html::Input;
use leptos::*;

type InputElement =
    (NodeRef<Input>, RwSignal<Option<String>>, RwSignal<String>);
pub type InputElements = HashMap<String, InputElement>;

#[component]
pub fn InputFieldView(
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

#[derive(Clone)]
pub struct InputData {
    pub value: String,
    pub validator: Arc<dyn Fn(&str) -> Result<(), String>>,
}

impl fmt::Debug for InputData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InputData")
            .field("value", &self.value)
            // Simply indicate presence of a validation function, since we can't print the function itself
            .field("validate", &true) // always true in current design
            .finish()
    }
}

impl InputData {
    fn new(
        value: String,
        validator: Arc<dyn Fn(&str) -> Result<(), String>>,
    ) -> Self {
        Self { value, validator }
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
        validate_fn: Arc<dyn Fn(&str) -> Result<(), String>>,
    ) -> Self {
        Self {
            name: name.to_string(),
            input_data: InputData::new(default, validate_fn),
        }
    }

    pub fn to_input_data(self) -> (String, InputData) {
        (self.name, self.input_data)
    }
}

pub fn create_input_elements(
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
