use std::sync::Arc;

use std::collections::HashMap;

use leptos::ev::SubmitEvent;
use leptos::*;
use regex::Regex;
use uuid::Uuid;

use lakestream::EnvironmentConfig;
use crate::base::connector::LakestreamHandler;

use crate::GlobalState;
use crate::builders::{
    ElementBuilder, FormBuilder, FormType, SubmitParameters,
};
use crate::components::forms::{ConfigurationFormMeta, FormData, FormError};
use crate::components::input::{validate_with_pattern, FieldContentType};
use crate::components::input::*;

const ENVIRONMENT_FORM_ID: &str = "EnvironmentForm";


#[component]
pub fn SearchForm(cx: Scope) -> impl IntoView {
    let is_submitting = create_rw_signal(cx, false);
    let validation_error = create_rw_signal(cx, None::<String>);

    // define results_form first as its the target for handle_search
    let form_meta = ConfigurationFormMeta::with_id(&Uuid::new_v4().to_string());
    let results_form =
        FormBuilder::new("Search Form", form_meta, FormType::LoadElements)
            .build(cx, None);

    // allows to overwrite the form
    let results_rw = results_form.form_data_rw();

    let handle_search = {
        move |ev: SubmitEvent, form_data: Option<FormData>| {
            ev.prevent_default();
            results_rw.set(None);
            is_submitting.set(true);

            spawn_local(async move {
                // run search on background
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

                    let query_params = form_data.export_config();

                    let memory_store = use_context::<RwSignal<GlobalState>>(cx)
                        .expect("state to have been provided")
                        .with(|state| state.store.clone());
                    let store = memory_store.lock().unwrap();
                    match store.load_config(ENVIRONMENT_FORM_ID).await {
                        Ok(Some(environment)) => {

                            let config = EnvironmentConfig {
                                settings: environment,
                            };

                            let handler = LakestreamHandler::new(config);
                            let uri = query_params.get("From").unwrap().to_string();
                            let max_files = 20; // TODO: get from User config

                            let results = handler.list_objects(uri, max_files).await;
                            log!("Results: {:?}", results);
                        }
                        Ok(None) => {
                            log!("No data found for form_id: {}", ENVIRONMENT_FORM_ID);
                        }
                        Err(e) => {
                            log!(
                                "Error loading data: {:?} for form_id: {}",
                                e,
                                ENVIRONMENT_FORM_ID
                            );
                        }
                    }

                } else {
                    log!("Form data is empty");
                }

                let form_data = make_form_data(cx);
                log!("Form data: {:?}", form_data);
                results_rw.set(Some(form_data));
                is_submitting.set(false);
            });
        }
    };

    let uri_pattern = Regex::new(r"^s3://").unwrap();

    let submit_parameters = SubmitParameters::new(
        Box::new(handle_search),
        Some(is_submitting),
        Some(validation_error),
        None,
    );

    let form_meta = ConfigurationFormMeta::with_id(&Uuid::new_v4().to_string());
    let mut query_form = FormBuilder::new(
        "Query",
        form_meta,
        FormType::SubmitData(submit_parameters),
    );

    query_form
        .add_element(
            ElementBuilder::new("Select", FieldContentType::PlainText)
                .with_label("Select")
                .with_placeholder("*")
                .with_initial_value("*"),
        )
        .add_element(
            ElementBuilder::new("From", FieldContentType::PlainText)
                .with_label("From")
                .with_placeholder("s3://bucket")
                .validator(Some(Arc::new(validate_with_pattern(
                    uri_pattern,
                    "Unsupported source".to_string(),
                )))),
        );

    let query_form = query_form.build(cx, None);

    view! { cx,
        { query_form.to_view() }
        { move ||
            if results_rw.get().is_none() {
                // submit not yet clicked
                view! { cx, ""}.into_view(cx)
            } else if let Some(error) = validation_error.get() {
                view! { cx, <p>{ error }</p> }.into_view(cx)
            } else {
                view ! {
                    cx,
                    <div>
                        <p>"Results"</p>
                    </div>
                    { results_form.to_view() }
                }.into_view(cx)
            }
        }
    }
    .into_view(cx)
}

pub fn make_form_elements() -> Vec<FormElement> {
    // textbox with validation
    let text_area_element = FormElement {
        field_content_type: FieldContentType::PlainText,
        field_label: None,
        field_placeholder: None,
        validator: None,
        buffer_data: "".to_string(),
        name: "TextAreaElement".to_string(),
        is_enabled: true,
    };

    let elements = vec![text_area_element];
    elements
}

pub fn make_form_data(cx: Scope) -> FormData {
    let elements = make_form_elements();
    let mut tags = HashMap::new();
    tags.insert("Name".to_string(), "Test Form".to_string());

    let form_meta = ConfigurationFormMeta::with_id("Form1").with_tags(tags);
    let form_data = FormData::build(cx, form_meta, &elements, None);
    form_data
}
