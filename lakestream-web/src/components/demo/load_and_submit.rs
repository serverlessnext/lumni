
use std::collections::HashMap;

use leptos::ev::SubmitEvent;
use leptos::*;
use uuid::Uuid;


use crate::builders::{
    FieldBuilder, FormBuilder, LoadParameters, SubmitParameters, FormType,
};
use crate::components::forms::{FormData, FormError};
use crate::components::form_input::{FormState, ElementDataType, DisplayValue};
use super::dummy_data::make_form_data;


#[component]
pub fn LoadAndSubmitDemo(cx: Scope) -> impl IntoView {
    let is_loading = create_rw_signal(cx, false);
    let is_submitting = create_rw_signal(cx, false);

    let load_error = create_rw_signal(cx, None::<String>);
    let submit_error = create_rw_signal(cx, None::<String>);

    // define a function that fetches the data
    let handle_load = {
        move |form_data_rw: RwSignal<Option<FormData>>| {
            // TODO:
            // - load elements instead of form_data
            // - filter out elements that are not in the form, copy attributes
            //   like validator or other non-data attributes from form elements,
            //   and then update elements with data
            // - may use FormData::build_with_config() instead of ::build(),
            //   or create new method

            spawn_local(async move {
                // run data loading on the background
                // overwrite all form-data
                let form_data = load_data(cx).await.unwrap();
                form_data_rw.set(Some(form_data));
                is_loading.set(false);
            });
        }
    };

    // define a function to handle form submission
    let handle_submit =
        move |ev: SubmitEvent, form_data: Option<FormData>| {
            ev.prevent_default();

            spawn_local(async move {
                if let Some(form_data) = form_data {
                    let form_state = form_data.form_state().clone();
                    let validation_errors =
                        perform_validation(&form_state);

                    if validation_errors.is_empty() {
                        log!("Form data is valid");
                    } else {
                        log!("Form data is invalid");
                        log!("Validation errors: {:?}", validation_errors);
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

    let load_and_submit_form = FormBuilder::new(
        "Load and Submit Form",
        &Uuid::new_v4().to_string(),
        FormType::LoadAndSubmitData(load_parameters, submit_parameters)
    )
    .add_element(Box::new(
        FieldBuilder::new("Select")
            .with_label("Select")
            .as_input_field()
            .with_initial_value("*"),
    ))
    .build(cx);

    load_and_submit_form.to_view()
}

async fn load_data(cx: Scope) -> Result<FormData, FormError> {
    // simulate high latency in debug mode
    #[cfg(feature = "debug-assertions")]
    crate::debug_sleep!();

    log!("Loading data...");
    Ok(make_form_data(cx))
}

async fn submit_data(
    _form_data: FormData,
) -> Result<(), FormError> {
    log!("Submitting data...");
    Ok(())
}


fn perform_validation(form_state: &FormState) -> HashMap<String, String> {
    let mut validation_errors = HashMap::new();
    for (key, element_state) in form_state {
        let value = element_state.read_display_value();
        let validator = match &element_state.schema.element_type {
            ElementDataType::TextData(text_data) => {
                text_data.validator.clone()
            }
            // Add other ElementDataType cases if they have a validator
            _ => None,
        };

        if let Some(validator) = validator {
            match &value {
                DisplayValue::Text(text) => {
                    if let Err(e) = validator(text) {
                        log::error!("Validation failed: {}", e);
                        validation_errors
                            .insert(key.clone(), e.to_string());
                    }
                }
                DisplayValue::Binary(_) => {
                    log::error!(
                        "Validation failed: Binary data cannot be \
                         validated."
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
