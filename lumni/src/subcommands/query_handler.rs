use std::sync::Arc;

use log::{debug, error};

use crate::{EnvironmentConfig, ObjectStoreHandler, TableCallback, TableRow};

pub async fn handle_query(
    query_matches: &clap::ArgMatches,
    config: &mut EnvironmentConfig,
) {
    // Retrieve the SQL statement from the command-line arguments
    let statement = query_matches
        .get_one::<String>("statement")
        .expect("SQL statement is required");

    let handler = ObjectStoreHandler::new(None);

    let callback = Arc::new(PrintCallback);
    // Execute the SQL query through the ObjectStoreHandler
    // Assuming `execute_query` can utilize the same `ListObjectsResult` for its output
    match handler
        .execute_query(statement, config, Some(callback))
        .await
    {
        Ok(_) => {
            debug!("Query executed successfully with no return value.");
        }
        Err(err) => {
            error!("Error executing query: {}", err);
            std::process::exit(1);
        }
    }
}
struct PrintCallback;
impl TableCallback for PrintCallback {
    fn on_row_add(&self, row: &mut TableRow) {
        row.print_columns();
    }
}
