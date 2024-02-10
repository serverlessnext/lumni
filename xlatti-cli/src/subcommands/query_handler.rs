use xlatti::{
    CallbackWrapper, EnvironmentConfig, ObjectStoreHandler,
};

use super::ls_handler::{
    handle_list_objects_result, print_callback_items_async,
};

pub async fn handle_query(
    query_matches: &clap::ArgMatches,
    config: &mut EnvironmentConfig,
) {
    // Retrieve the SQL statement from the command-line arguments
    let statement = query_matches
        .get_one::<String>("statement")
        .expect("SQL statement is required");

    let handler = ObjectStoreHandler::new(None);

    // Reusing the callback mechanism for async processing
    let callback =
        Some(CallbackWrapper::create_async(print_callback_items_async));

    // Execute the SQL query through the ObjectStoreHandler
    // Assuming `execute_query` can utilize the same `ListObjectsResult` for its output
    match handler.execute_query(statement, config, callback).await {
        Ok(Some(query_result)) => {
            // Reuse the existing result handling logic
            handle_list_objects_result(query_result).await;
        }
        Ok(None) => {
            println!("Query executed successfully with no return value.");
        }
        Err(err) => {
            eprintln!("Error executing query: {:?}", err);
        }
    }
}
