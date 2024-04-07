use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use futures::channel::mpsc;
use futures::stream::StreamExt;

use crate::api::error::*;
use crate::api::handler::AppHandler;
use crate::api::invoke::{Request, Response};
use crate::api::types::{
    Column, ColumnarData, ColumnarTable, Data, DataType, RowTable, Table,
};
use crate::base::connector::LakestreamHandler;
use crate::utils::string_replace::replace_variables_in_string_with_map;
use crate::EnvironmentConfig;

#[derive(Clone)]
pub struct Handler;

impl AppHandler for Handler {
    fn clone_box(&self) -> Box<dyn AppHandler> {
        Box::new(self.clone())
    }
    fn process_request(
        &self,
        rx: mpsc::UnboundedReceiver<Request>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>>>> {
        Box::pin(handle_query(rx))
    }

    fn handle_runtime(
        &self,
        _args: Vec<String>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send>> {
        Box::pin(async move {
            Ok(())
        })
    }
    
    fn load_config(&self) -> &str {
        include_str!("spec.yaml")
    }
}

pub async fn handle_query(
    mut rx: mpsc::UnboundedReceiver<Request>,
) -> Result<(), Error> {
    if let Some(request) = rx.next().await {
        let tx = request.tx();

        let response = match request.content() {
            Data::KeyValue(kv) => {
                let query = kv.get_string_or_default("QUERY", "");

                let config =
                    request.config().unwrap_or_else(EnvironmentConfig::default);
                let settings = config.get_settings();
                let query =
                    replace_variables_in_string_with_map(&query, settings);
                log::info!("Query: {}", query);
                let handler = LakestreamHandler::new(config);
                let results = handler.execute_query(query).await;
                log::info!("Results: {:?}", results);

                // TODO: wrap results into rows and columns
                Ok(generate_test_data_row())
            }
            _ => {
                let err = Error::Request(RequestError::QueryInvalid(
                    "Invalid data type".into(),
                ));
                Err(err)
            }
        };

        tx.unbounded_send(response).unwrap();
    }
    Ok(())
}

#[allow(dead_code)]
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
