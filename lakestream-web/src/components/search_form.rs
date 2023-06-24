use std::collections::HashMap;

use leptos::ev::SubmitEvent;
use leptos::*;
use uuid::Uuid;

use crate::components::form_input::FieldBuilder;
use crate::components::forms::{FormBuilder, FormData, FormError};

#[component]
pub fn SearchForm(cx: Scope) -> impl IntoView {
    let is_submitting = create_rw_signal(cx, false);
    let validation_error = create_rw_signal(cx, None::<String>);

    let handle_search = {
        move |ev: SubmitEvent, form_data: Option<FormData>| {
            ev.prevent_default();

            log!("Button clicked");

            let data = extract_form_data(form_data)
                .map_err(|e| {
                    log!("Error: {:?}", e);
                    validation_error.set(Some("FORM_DATA_MISSING".to_string()));
                })
                .unwrap();

            log!("Form data: {:?}", data);
            validation_error.set(Some("random error".to_string()));
            is_submitting.set(false);
        }
    };

    let form = FormBuilder::new("Search Form", &Uuid::new_v4().to_string())
        .add_element(Box::new(
            FieldBuilder::new("field1")
                .as_input_field()
                .with_initial_value("foo"),
        ))
        .add_element(Box::new(
            FieldBuilder::new("field2")
                .as_input_field()
                .with_initial_value("bar"),
        ))
        .on_submit(Box::new(handle_search), is_submitting, validation_error)
        .build(cx);

    form.to_view()
}

fn extract_form_data(
    form_data: Option<FormData>,
) -> Result<HashMap<String, String>, FormError> {
    let data = form_data
        .ok_or_else(|| FormError::SubmitError("FORM_DATA_MISSING".to_string()))
        .map(|data| data.to_hash_map())?;
    Ok(data)
}
