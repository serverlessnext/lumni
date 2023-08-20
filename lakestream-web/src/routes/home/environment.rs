use std::collections::HashMap;

use leptos::ev::SubmitEvent;
use leptos::*;

use crate::GlobalState;
use crate::builders::{
    FormType, LoadParameters, ProfileFormBuilder, SubmitParameters,
};
use crate::components::buttons::{ButtonType, FormButton};
use crate::components::forms::{ConfigurationFormMeta, FormData};
use crate::components::input::*;

const ENVIROMENT_FORM_ID: &str = "EnvironmentForm";

#[component]
pub fn Environment(cx: Scope) -> impl IntoView {

    let is_loading = create_rw_signal(cx, false);
    let load_error = create_rw_signal(cx, None::<String>);
    let validation_error = create_rw_signal(cx, None::<String>);

    let is_submitting = create_rw_signal(cx, false);
    let submit_error = create_rw_signal(cx, None::<String>);

    let handle_load = {

        let memory_store = use_context::<RwSignal<GlobalState>>(cx)
            .expect("state to have been provided")
            .with(|state| state.store.clone());

        move |form_data_rw: RwSignal<Option<FormData>>| {
            let memory_store = memory_store.clone();
            is_loading.set(true);
            spawn_local(async move {
                let store = memory_store.lock().unwrap();
                match store.load_config(ENVIROMENT_FORM_ID).await {
                    Ok(Some(config)) => {
                        log!("Data loaded for form_id: {}", ENVIROMENT_FORM_ID);
                        let mut form_data = form_data_rw.get_untracked().unwrap();
                        form_data.update_with_config(config);
                        form_data_rw.set(Some(form_data));
                        is_loading.set(false);
                    }
                    Ok(None) => {
                        log!("No data found for form_id: {}", ENVIROMENT_FORM_ID);
                        is_loading.set(false);
                    }
                    Err(e) => {
                        log!(
                            "Error loading data: {:?} for form_id: {}",
                            e,
                            ENVIROMENT_FORM_ID
                        );
                        load_error.set(Some(e));
                        is_loading.set(false);
                    }
                }
            });
        }
    };

    let handle_submit = {
        let memory_store = use_context::<RwSignal<GlobalState>>(cx)
            .expect("state to have been provided")
            .with(|state| state.store.clone());

        move |ev: SubmitEvent, form_data: Option<FormData>| {
            ev.prevent_default();
            is_submitting.set(true);
            let memory_store = memory_store.clone();

            spawn_local(async move {
                let store = memory_store.lock().unwrap();
                if let Some(form_data) = form_data {
                    let form_elements = form_data.elements();
                    let validation_errors = perform_validation(&form_elements);
                    if validation_errors.is_empty() {
                        let result = store.save_config(&form_data).await;
                        match result {
                            Ok(_) => {
                                log!("Data submitted successfully");
                                is_submitting.set(false);
                            }
                            Err(e) => {
                                log!("Data submission failed: {:?}", e);
                                submit_error.set(Some(e.to_string()));
                            }
                        }
                    } else {
                        log!("Form data is invalid");
                        log!("Validation errors: {:?}", validation_errors);
                    }
                } else {
                    log!("Form data is empty");
                }
                is_submitting.set(false);
            });
        }
    };

    let load_parameters = LoadParameters::new(
        Some(Box::new(handle_load)),
        Some(is_loading),
        Some(validation_error),
    );

    let save_button =
        FormButton::new(ButtonType::Save, None).set_enabled(false);
    let submit_parameters = SubmitParameters::new(
        Box::new(handle_submit),
        Some(is_submitting),
        Some(submit_error),
        Some(save_button),
    );

    let form_meta = ConfigurationFormMeta::with_id(ENVIROMENT_FORM_ID);
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
