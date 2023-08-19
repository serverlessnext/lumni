use std::collections::HashMap;

use leptos::ev::SubmitEvent;
use leptos::*;
use uuid::Uuid;

use crate::builders::{
    FormType, LoadParameters, ProfileFormBuilder, SubmitParameters,
};
use crate::components::forms::{ConfigurationFormMeta, FormData};
use crate::components::input::*;

#[component]
pub fn Environment(cx: Scope) -> impl IntoView {
    let is_loading = create_rw_signal(cx, false);
    // let load_error = create_rw_signal(cx, None::<String>);
    let validation_error = create_rw_signal(cx, None::<String>);

    let is_submitting = create_rw_signal(cx, false);
    let submit_error = create_rw_signal(cx, None::<String>);

    // define a function that fetches the data
    let handle_load = {
        let dummy_data = make_form_data(cx);
        move |form_data_rw: RwSignal<Option<FormData>>| {
            let dummy_data = dummy_data.clone();
            is_loading.set(true);
            spawn_local(async move {
                // run data loading on the background
                form_data_rw.set(Some(dummy_data));
                is_loading.set(false);
            });
        }
    };

    let handle_submit = move |ev: SubmitEvent, form_data: Option<FormData>| {
        ev.prevent_default();

        spawn_local(async move {
            if let Some(form_data) = form_data {
                let form_elements = form_data.elements();
                let validation_errors = perform_validation(form_elements);

                if validation_errors.is_empty() {
                    log!("Form data is valid");
                } else {
                    log!("Form data is invalid");
                    log!("Validation errors: {:?}", validation_errors);
                    is_submitting.set(false);
                    return;
                }
                log!("Submitting form data: {:?}", form_data);
            //                match result {
            //                    Ok(_) => log!("Data submitted successfully"),
            //                    Err(e) => log!("Data submission failed: {:?}", e),
            //                }
            } else {
                log!("Form data is empty");
            }
            is_submitting.set(false);
        });
    };
    let load_parameters = LoadParameters::new(
        Some(Box::new(handle_load)),
        Some(is_loading),
        Some(validation_error),
    );

    let submit_parameters = SubmitParameters::new(
        Box::new(handle_submit),
        Some(is_submitting),
        Some(submit_error),
        None,
    );

    let form_meta = ConfigurationFormMeta::with_id(&Uuid::new_v4().to_string());
    let form = ProfileFormBuilder::new(
        "Load Form",
        form_meta,
        FormType::LoadAndSubmitData(load_parameters, submit_parameters),
    )
    .to_text_area()
    .build(cx);

    form.to_view()
}

pub fn make_form_data(cx: Scope) -> FormData {
    let text_area_element = FormElement {
        field_content_type: FieldContentType::PlainText,
        field_label: Some(FieldLabel::new("Text Area")),
        validator: None,
        buffer_data: "type anything".to_string(),
        name: "TextAreaElement".to_string(),
        is_enabled: true,
    };

    let elements = vec![text_area_element];
    let mut tags = HashMap::new();
    tags.insert("Name".to_string(), "Test Form".to_string());

    let form_meta = ConfigurationFormMeta::with_id("Form1").with_tags(tags);
    let form_data = FormData::build(cx, form_meta, &elements, None);
    form_data
}
