use std::collections::HashMap;
use std::sync::Arc;

use futures::channel::mpsc;
use futures::stream::StreamExt;
use lakestream::EnvironmentConfig;
use leptos::ev::SubmitEvent;
use leptos::*;
use regex::Regex;
use uuid::Uuid;

use crate::api::error::*;
use crate::api::invoke::{Request, Response};
use crate::api::types::{
    Column, ColumnarData, ColumnarTable, Data, DataType, RowTable, Table,
    TableType,
};
use crate::base::connector::LakestreamHandler;
use crate::components::builders::{
    ElementBuilder, FormBuilder, FormType, SubmitParameters,
};
use crate::components::forms::{ConfigurationFormMeta, FormData};
use crate::components::input::{
    perform_validation, validate_with_pattern, FieldContentType,
};
use crate::GlobalState;

const ENVIRONMENT_FORM_ID: &str = "EnvironmentForm";

#[component]
pub fn AppFormSubmit(cx: Scope) -> impl IntoView {
    let is_submitting = create_rw_signal(cx, false);
    let validation_error = create_rw_signal(cx, None::<String>);

    let form_meta = ConfigurationFormMeta::with_id(&Uuid::new_v4().to_string());
    let results_form =
        FormBuilder::new("Search Form", form_meta, FormType::LoadElements)
            .build(cx, None);

    let results_rw = results_form.form_data_rw();

    let memory_store = use_context::<RwSignal<GlobalState>>(cx)
        .expect("state to have been provided")
        .with(|state| state.store.clone());

    let (tx, mut rx) = mpsc::unbounded::<Result<Response, Error>>();

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
                    Error::Request(RequestError::ConfigInvalid(e)) => {
                        log::error!("Request Error - Invalid Config: {}", e);
                    }
                    Error::Request(RequestError::QueryInvalid(e)) => {
                        log::error!("Request Error - Invalid Query: {}", e);
                    }
                    Error::Runtime(RuntimeError::Unexpected(e)) => {
                        log::error!("Runtime Error - Unexpected: {}", e);
                    }
                },
            }
        }
    });

    let handle_submit = {
        move |ev: SubmitEvent, form_data: Option<FormData>| {
            let memory_store = memory_store.clone();
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
                            Ok(Some(environment)) => Some(EnvironmentConfig {
                                settings: environment,
                            }),
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

                        handle_query(rx_handler).await;

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

    let builders = vec![
        ElementBuilder::new("Select", FieldContentType::PlainText)
            .with_label("Select")
            .with_placeholder("*")
            .with_initial_value("*"),
        ElementBuilder::new("From", FieldContentType::PlainText)
            .with_label("From")
            .with_placeholder("s3://bucket")
            .validator(Some(Arc::new(validate_with_pattern(
                Regex::new(r"^s3://").unwrap(),
                "Unsupported source".to_string(),
            )))),
    ];
    let query_form = query_form.with_elements(builders).build(cx, None);

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

async fn handle_query(mut rx: mpsc::UnboundedReceiver<Request>) {
    if let Some(request) = rx.next().await {
        log!("Received query");

        let config = request.config();
        let content = request.content();
        let tx = request.tx();

        // Note: I'm assuming the Response type can carry either Data or an Error.
        let response: Result<Response, Error>;

        if let Some(conf) = config {
            let handler = LakestreamHandler::new(conf);

            match &content {
                Data::KeyValue(kv) => {
                    let select_string = kv.get_string_or_default("Select", "*");
                    let query_uri = kv.get_string_or_default("From", "s3://");
                    log!("Select {} From {}", select_string, query_uri);

                    let max_files = 20; // TODO: get query
                    let results =
                        handler.list_objects(query_uri, max_files).await;
                    log!("Results: {:?}", results);

                    // TODO: wrap results into rows and columns
                    response = Ok(generate_test_data_row());
                }
                _ => {
                    let err = Error::Request(RequestError::QueryInvalid(
                        "Invalid data type".into(),
                    ));
                    response = Err(err);
                }
            }
        } else {
            let err = Error::Request(RequestError::ConfigInvalid(
                "No config provided".into(),
            ));
            response = Err(err);
        }

        tx.unbounded_send(response).unwrap();
    }
}

fn generate_test_data_columnar() -> Response {
    // Define column data
    let names_column_data = ColumnarData {
        data_type: DataType::String(String::new()),
        data: vec![
            Some(DataType::String("Jane Smith".to_string())),
            Some(DataType::String("Robert Brown".to_string())),
        ],
    };

    let age_column_data = ColumnarData {
        data_type: DataType::Integer32(0),
        data: vec![
            Some(DataType::Integer32(25)),
            None, // Null value
        ],
    };

    let verified_column_data = ColumnarData {
        data_type: DataType::Boolean(true),
        data: vec![
            None, // Null value
            Some(DataType::Boolean(true)),
        ],
    };

    // Create a hashmap for the columns
    let mut columns = HashMap::new();
    columns.insert("Name".to_string(), names_column_data);
    columns.insert("Age".to_string(), age_column_data);
    columns.insert("Verified".to_string(), verified_column_data);

    // Create the ColumnarTable
    let table = ColumnarTable { columns };

    let response =
        Response::new(Data::Table(Arc::new(table) as Arc<dyn Table>));
    response
}

fn generate_test_data_row() -> Response {
    let columns = vec![
        Column {
            name: "Name".to_string(),
            data_type: DataType::String("".to_string()),
        },
        Column {
            name: "Age".to_string(),
            data_type: DataType::Integer32(0),
        },
    ];

    // Define rows
    let rows = vec![
        vec![
            Some(DataType::String("Alice".to_string())),
            Some(DataType::Integer32(30)),
        ],
        vec![
            Some(DataType::String("Bob".to_string())),
            Some(DataType::Integer32(25)),
        ],
        vec![None, Some(DataType::Integer32(22))], /* Note the None value for the "Name" field */
    ];

    // Create row-oriented table
    let row_table = RowTable { columns, rows };

    let response =
        Response::new(Data::Table(Arc::new(row_table) as Arc<dyn Table>));
    response
}
