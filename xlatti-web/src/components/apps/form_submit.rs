use std::sync::Arc;

use futures::channel::mpsc;
use futures::stream::StreamExt;
use leptos::ev::SubmitEvent;
use leptos::logging::log;
use leptos::*;
use uuid::Uuid;
use xlatti::EnvironmentConfig;

use crate::api::error::*;
use crate::api::invoke::{Request, Response};
use crate::api::types::{Data, TableType};
use crate::components::apps::configuration::AppConfig;
use crate::components::forms::builders::{
    FormBuilder, FormType, SubmitParameters,
};
use crate::components::forms::input::perform_validation;
use crate::components::forms::{ConfigurationFormMeta, FormData};
use crate::GlobalState;

const ENVIRONMENT_FORM_ID: &str = "EnvironmentForm";

#[component]
pub fn AppFormSubmit(app_uri: String) -> impl IntoView {
    let is_submitting = create_rw_signal(false);
    let validation_error = create_rw_signal(None::<String>);

    let form_meta = ConfigurationFormMeta::with_id(&Uuid::new_v4().to_string());
    let results_form =
        FormBuilder::new("Search Form", form_meta, FormType::LoadElements)
            .build(None);

    let results_rw = results_form.form_data_rw();

    let (tx, mut rx) = mpsc::unbounded::<Result<Response, Error>>();

    let app_config =
        AppConfig::new(app_uri, Some("Search Form".to_string()), None);

    let form_elements = match app_config {
        Some(ref config) => match config.configuration_form_elements() {
            Ok(elements) => elements,
            Err(_) => {
                log!("Error loading form elements");
                vec![] // Using an empty vector as a fallback
            }
        },
        None => vec![], // AppConfig is None, also use an empty vector as a fallback
    };
    log!("App Form Elements: {:?}", form_elements);
    //log!("AppConfig: {:?}", app_config);
    // TODO: handle None

    spawn_local(async move {
        while let Some(result) = rx.next().await {
            match result {
                Ok(response) => match response.content() {
                    Data::Empty => {
                        log!("Received an empty message");
                    }
                    Data::Table(table) => match table.table_type() {
                        TableType::Row => {
                            if let Some(row_table) = table.as_row() {
                                log!("Received row-based table");
                                log!("Columns: {:?}", row_table.columns);
                                log!("Rows: {:?}", row_table.rows);
                            }
                        }
                        TableType::Columnar => {
                            if let Some(columnar_table) = table.as_columnar() {
                                log!("Received columnar table");
                                for (column_name, column_data) in
                                    &columnar_table.columns
                                {
                                    log!("Column Name: {}", column_name);
                                    log!(
                                        "Column Data Type: {:?}",
                                        column_data.data_type
                                    );
                                    log!("Column Data: {:?}", column_data.data);
                                }
                            }
                        }
                    },
                    Data::Binary { data, metadata } => {
                        log!("Received binary data of length: {}", data.len());
                        if let Some(meta) = metadata {
                            log!("Metadata: {:?}", meta);
                        } else {
                            log!("No metadata provided");
                        }
                    }
                    Data::KeyValue(_key_value_type) => {
                        log!("Received key-value data");
                    }
                },
                Err(error) => match error {
                    Error::Request(RequestError::QueryInvalid(e)) => {
                        log::error!("Request Error - Invalid Query: {}", e);
                    }
                    Error::Runtime(RuntimeError::Unexpected(e)) => {
                        log::error!("Runtime Error - Unexpected: {}", e);
                    }
                    Error::Application(ApplicationError::ConfigInvalid(e)) => {
                        log::error!(
                            "Application Error - Invalid Config: {}",
                            e
                        );
                    }
                    Error::Application(ApplicationError::Unexpected(e)) => {
                        log::error!("Application Error - Unexpected: {}", e);
                    }
                },
            }
        }
    });

    let handle_submit = {
        let app_config_clone = app_config.clone();
        move |ev: SubmitEvent, form_data: Option<FormData>| {
            let app_config = app_config_clone.clone();

            let memory_store = use_context::<RwSignal<GlobalState>>()
                .expect("state to have been provided")
                .with(|state| state.store.clone());

            ev.prevent_default();
            results_rw.set(None);
            is_submitting.set(true);

            let form_elements_valid = if let Some(form_data) = &form_data {
                let form_elements = form_data.elements();
                let validation_errors = perform_validation(form_elements);

                if validation_errors.is_empty() {
                    log!("Form data is valid");
                    true
                } else {
                    log!("Form data is invalid");
                    log!("Validation errors: {:?}", validation_errors);
                    is_submitting.set(false);
                    false
                }
            } else {
                log!("Form data is empty");
                false
            };

            if form_elements_valid {
                if let Some(form_data) = form_data {
                    let query_params = form_data.export_config();

                    let tx_clone = tx.clone();
                    let (tx_handler, rx_handler) = mpsc::unbounded::<Request>();

                    spawn_local(async move {
                        let store = memory_store.lock().unwrap();
                        let config = match store
                            .load_config(ENVIRONMENT_FORM_ID)
                            .await
                        {
                            Ok(Some(environment)) => {
                                Some(EnvironmentConfig::new(environment))
                            }
                            Ok(None) => {
                                log!(
                                    "No data found for form_id: {}",
                                    ENVIRONMENT_FORM_ID
                                );
                                None
                            }
                            Err(e) => {
                                log!(
                                    "Error loading data: {:?} for form_id: {}",
                                    e,
                                    ENVIRONMENT_FORM_ID
                                );
                                is_submitting.set(false);
                                return; // Exit early if there's an error
                            }
                        };

                        tx_handler
                            .unbounded_send(Request::new(
                                Data::KeyValue(Arc::new(query_params)),
                                config,
                                tx_clone,
                            ))
                            .unwrap();

                        if let Some(config) = app_config {
                            config.handler().handle_query(rx_handler).await;
                        } else {
                            // Handle the case where app_config is None if necessary
                            log!("AppConfig is not available.");
                            is_submitting.set(false);
                        }

                        // query is done
                        is_submitting.set(false);
                    });
                }
            }
        }
    };

    let submit_parameters = SubmitParameters::new(
        Box::new(handle_submit),
        Some(is_submitting),
        Some(validation_error),
        None,
    );

    let form_meta = ConfigurationFormMeta::with_id(&Uuid::new_v4().to_string());
    let query_form = FormBuilder::new(
        "Query",
        form_meta,
        FormType::SubmitData(submit_parameters),
    );

    let interface_elements =
        match app_config.map(|config| config.interface_form_elements()) {
            Some(Ok(elements)) => elements,
            Some(Err(_)) => {
                log!("Error loading form elements");
                vec![] // fallback to an empty vector
            }
            None => vec![], // AppConfig is None, use an empty vector as a fallback
        };

    let query_form = query_form.with_elements(interface_elements).build(None);

    view! {
        { query_form.to_view() }
        { move ||
            if results_rw.get().is_none() {
                view! { ""}.into_view()
            } else if let Some(error) = validation_error.get() {
                view! { <p>{ error }</p> }.into_view()
            } else {
                view ! {
                    <div>
                        <p>"Results"</p>
                    </div>
                    { results_form.to_view() }
                }.into_view()
            }
        }
    }
    .into_view()
}
