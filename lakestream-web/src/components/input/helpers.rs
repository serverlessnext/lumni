use std::collections::HashMap;

use leptos::*;
use regex::Regex;

use super::DisplayValue;
use crate::components::forms::FormElements;

pub fn validate_with_pattern(
    pattern: Regex,
    error_msg: String,
) -> Box<dyn Fn(&str) -> Result<(), String>> {
    Box::new(move |input: &str| {
        if pattern.is_match(input) {
            Ok(())
        } else {
            Err(error_msg.clone())
        }
    })
}

pub fn perform_validation(
    form_elements: &FormElements,
) -> HashMap<String, String> {
    let mut validation_errors = HashMap::new();
    for (key, element_state) in form_elements {
        let value = element_state.read_display_value();
        let validator = element_state.schema.validator.clone();

        if let Some(validator) = validator {
            match &value {
                DisplayValue::Text(text) => {
                    if let Err(e) = validator(text) {
                        log::error!("Validation failed: {}", e);
                        validation_errors.insert(key.clone(), e.to_string());
                    }
                }
            }
        }
    }

    // Write validation errors to corresponding WriteSignals
    for (key, element_state) in form_elements {
        if let Some(error) = validation_errors.get(key) {
            element_state.display_error.set(Some(error.clone()));
        } else {
            element_state.display_error.set(None);
        }
    }
    validation_errors
}
