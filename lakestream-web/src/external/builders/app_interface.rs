use std::collections::HashMap;
use std::sync::Arc;

use futures::channel::mpsc;
use futures::stream::StreamExt;

use lakestream::EnvironmentConfig;
use leptos::ev::SubmitEvent;
use leptos::*;
use regex::Regex;
use uuid::Uuid;

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

#[derive(Debug)]
enum DataType {
    String(String),
    Integer(i32),
    Float(f64),
}

#[derive(Debug)]
struct Column {
    name: String,
    data_type: DataType,
}

type Row = Vec<DataType>;

enum Message {
    Results {
        columns: Vec<Column>,
        rows: Vec<Row>,
    },
}

struct HandlerMessage {
    query_params: HashMap<String, String>,
    config: Option<EnvironmentConfig>,
    tx: mpsc::UnboundedSender<Message>,
}


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


    let (tx, mut rx) = mpsc::unbounded::<Message>();

    spawn_local(async move {
        while let Some(message) = rx.next().await {
            match message {
                Message::Results { columns, rows } => {
                    // results_rw.set(data);
                    log!("Received results");
                    log!("Columns: {:?}", columns);
                    log!("Rows: {:?}", rows);
                }
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
                    let (tx_handler, rx_handler) = mpsc::unbounded::<HandlerMessage>();

                    spawn_local(async move {
                        let store = memory_store.lock().unwrap();
                        let config = match store.load_config(ENVIRONMENT_FORM_ID).await {
                            Ok(Some(environment)) => {
                                Some(EnvironmentConfig {
                                    settings: environment,
                                })
                            }
                            Ok(None) => {
                                log!("No data found for form_id: {}", ENVIRONMENT_FORM_ID);
                                None
                            }
                            Err(e) => {
                                log!("Error loading data: {:?} for form_id: {}", e, ENVIRONMENT_FORM_ID);
                                is_submitting.set(false);
                                return; // Exit early if there's an error
                            }
                        };

                        tx_handler.unbounded_send(HandlerMessage {
                            config,
                            query_params: query_params.clone(),
                            tx: tx_clone,
                        }).unwrap();

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
            ))))
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

async fn handle_query(mut rx: mpsc::UnboundedReceiver<HandlerMessage>) {
    if let Some(message) = rx.next().await {
        log!("Received query");
        let HandlerMessage { config, query_params, tx } = message;

        let message = Message::Results {
            columns: Vec::new(),
            rows: Vec::new(),
        };

        if let Some(conf) = config {
            let handler = LakestreamHandler::new(conf);
            let uri = query_params.get("From").unwrap().to_string();
            let max_files = 20; // TODO: get from User config

            let results = handler.list_objects(uri, max_files).await;
            log!("Results: {:?}", results);

            // TODO: wrap results into rows and columns
        } else {
            log!("No config provided. Skipping query handling.");
        }
        tx.unbounded_send(message).unwrap();
    }
}

