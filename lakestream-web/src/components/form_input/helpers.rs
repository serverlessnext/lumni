use std::collections::HashMap;

use leptos::*;
use regex::Regex;

use crate::components::form_input::{DisplayValue, ElementDataType, FormState};

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

pub fn perform_validation(form_state: &FormState) -> HashMap<String, String> {
    let mut validation_errors = HashMap::new();
    for (key, element_state) in form_state {
        let value = element_state.read_display_value();
        let validator = match &element_state.schema.element_type {
            ElementDataType::TextData(text_data) => text_data.validator.clone(),
            // Add other ElementDataType cases if they have a validator
            _ => None,
        };

        if let Some(validator) = validator {
            match &value {
                DisplayValue::Text(text) => {
                    if let Err(e) = validator(text) {
                        log::error!("Validation failed: {}", e);
                        validation_errors.insert(key.clone(), e.to_string());
                    }
                }
                DisplayValue::Binary(_) => {
                    log::error!(
                        "Validation failed: Binary data cannot be validated."
                    );
                    validation_errors.insert(
                        key.clone(),
                        "Binary data cannot be validated.".to_string(),
                    );
                }
            }
        }
    }

    // Write validation errors to corresponding WriteSignals
    for (key, element_state) in form_state {
        if let Some(error) = validation_errors.get(key) {
            element_state.display_error.set(Some(error.clone()));
        } else {
            element_state.display_error.set(None);
        }
    }
    validation_errors
}
