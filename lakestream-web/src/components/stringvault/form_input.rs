use std::fmt;
use std::sync::Arc;

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
