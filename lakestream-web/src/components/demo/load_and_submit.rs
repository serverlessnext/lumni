use std::collections::HashMap;
use std::sync::Arc;

use leptos::ev::SubmitEvent;
use leptos::*;
use regex::Regex;
use uuid::Uuid;

use super::dummy_data::make_form_data;
use crate::builders::{
    ElementBuilder, FormBuilder, FormType, LoadParameters, SubmitParameters,
};
use crate::components::forms::{FormData, FormError};
use crate::components::input::{
    validate_with_pattern, DisplayValue, FieldContentType,
    FormState,
};

#[component]
pub fn LoadAndSubmitDemo(cx: Scope) -> impl IntoView {
    let is_loading = create_rw_signal(cx, false);
    let is_submitting = create_rw_signal(cx, false);

    let load_error = create_rw_signal(cx, None::<String>);
    let submit_error = create_rw_signal(cx, None::<String>);

    // define a function that fetches the data
    let handle_load = {
        move |form_data_rw: RwSignal<Option<FormData>>| {
            spawn_local(async move {
                // run data loading on the background
                let mut form_data = form_data_rw.get_untracked().unwrap();

                let config = make_update_config();
                form_data.update_with_config(config);

                form_data_rw.set(Some(form_data));
                is_loading.set(false);
            });
        }
    };

    // define a function to handle form submission
    let handle_submit = move |ev: SubmitEvent, form_data: Option<FormData>| {
        ev.prevent_default();

        spawn_local(async move {
            if let Some(form_data) = form_data {
                let form_state = form_data.form_state().clone();
                let validation_errors = perform_validation(&form_state);

                if validation_errors.is_empty() {
                    log!("Form data is valid");
                } else {
                    log!("Form data is invalid");
                    log!("Validation errors: {:?}", validation_errors);
                    is_submitting.set(false);
                    return;
                }

                let result = submit_data(form_data).await;
                match result {
                    Ok(_) => log!("Data submitted successfully"),
                    Err(e) => log!("Data submission failed: {:?}", e),
                }
            } else {
                log!("Form data is empty");
            }
            is_submitting.set(false);
        });
    };

    let load_parameters = LoadParameters::new(
        Some(Box::new(handle_load)),
        Some(is_loading),
        Some(load_error),
    );

    let submit_parameters = SubmitParameters::new(
        Box::new(handle_submit),
        Some(is_submitting),
        Some(submit_error),
        None,
    );

    let foo_pattern = Regex::new(r"^foo$").unwrap();
    let validate_foo = Arc::new(validate_with_pattern(
        foo_pattern,
        "Input can only be foo".to_string(),
    ));

    let mut load_and_submit_form = FormBuilder::new(
        "Load and Submit Form",
        &Uuid::new_v4().to_string(),
        None,
        FormType::LoadAndSubmitData(load_parameters, submit_parameters),
    );

    load_and_submit_form.add_element(
        ElementBuilder::new("Select", FieldContentType::PlainText)
            .with_label("Select")
            .with_initial_value("*")
            .validator(Some(validate_foo)),
    );

    let load_and_submit_form = load_and_submit_form.build(cx);

    load_and_submit_form.to_view()
}

async fn load_data(cx: Scope) -> Result<FormData, FormError> {
    // simulate high latency in debug mode
    #[cfg(feature = "debug-assertions")]
    crate::debug_sleep!();

    log!("Loading data...");
    Ok(make_form_data(cx))
}

async fn submit_data(_form_data: FormData) -> Result<(), FormError> {
    log!("Submitting data...");
    Ok(())
}

fn perform_validation(form_state: &FormState) -> HashMap<String, String> {
    let mut validation_errors = HashMap::new();
    for (key, element_state) in form_state.elements() {
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
    for (key, element_state) in form_state.elements() {
        if let Some(error) = validation_errors.get(key) {
            element_state.display_error.set(Some(error.clone()));
        } else {
            element_state.display_error.set(None);
        }
    }
    validation_errors
}

fn make_update_config() -> HashMap<String, String> {
    let mut config = HashMap::new();
    config.insert("Select".to_string(), "Test update".to_string());
    config
}
