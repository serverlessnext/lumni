
use std::sync::Arc;
use super::{InputData, InputElementOpts, FormInputField};

#[derive(Clone, Default)]
pub struct FormInputFieldBuilder {
    name: String,
    default: String,
    opts: InputElementOpts,
    validate_fn: Option<Arc<dyn Fn(&str) -> Result<(), String>>>,
}

impl FormInputFieldBuilder {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..Default::default()
        }
    }

    pub fn default(mut self, default: String) -> Self {
        self.default = default;
        self
    }

    pub fn secret(mut self, is_secret: bool) -> Self {
        self.opts.is_secret = is_secret;
        self
    }

    pub fn enabled(mut self, is_enabled: bool) -> Self {
        self.opts.is_enabled = is_enabled;
        self
    }

    pub fn validator(mut self, validate_fn: Option<Arc<dyn Fn(&str) -> Result<(), String>>>) -> Self {
        self.validate_fn = validate_fn;
        self
    }

    pub fn build(self) -> FormInputField {
        FormInputField {
            name: self.name,
            input_data: InputData::new(self.default, self.opts, self.validate_fn),
        }
    }
}
