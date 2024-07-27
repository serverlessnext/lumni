use std::sync::Arc;

use log::{debug, error};
use lumni::{
    EnvironmentConfig, IgnoreContents, ObjectStoreHandler, TableCallback,
    TableRow,
};

pub async fn handle_query(
    query_matches: &clap::ArgMatches,
    config: &mut EnvironmentConfig,
) {
    let no_recursive = *query_matches
        .get_one::<bool>("no_recursive")
        .unwrap_or(&false);
    let show_hidden = *query_matches
        .get_one::<bool>("show_hidden")
        .unwrap_or(&false);
    let no_gitignore = *query_matches
        .get_one::<bool>("no_gitignore")
        .unwrap_or(&false);
    let other_ignore_files = query_matches
        .get_many::<String>("other_ignore_files")
        .map_or(Vec::new(), |vals| {
            vals.map(String::from).collect::<Vec<_>>()
        });

    let ignore_contents =
        Some(IgnoreContents::new(other_ignore_files, !no_gitignore));

    // Retrieve the SQL statement from the command-line arguments
    let statement = query_matches
        .get_one::<String>("statement")
        .expect("SQL statement is required");

    let handler = ObjectStoreHandler::new(None);

    let callback = Arc::new(PrintCallback);
    // Execute the SQL query through the ObjectStoreHandler
    // Assuming `execute_query` can utilize the same `ListObjectsResult` for its output
    match handler
        .execute_query(
            statement,
            config,
            !show_hidden,
            !no_recursive,
            ignore_contents,
            Some(callback),
        )
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
