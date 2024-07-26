use std::sync::Arc;

use log::{debug, error};
use lumni::{
    EnvironmentConfig, FileObjectFilter, InternalError, ObjectStoreHandler,
    ParsedUri, TableCallback, TableRow,
};

pub async fn handle_ls(
    ls_matches: &clap::ArgMatches,
    config: &mut EnvironmentConfig,
) {
    let (uri, recursive, max_files, filter) =
        prepare_handle_ls_arguments(ls_matches);

    let handler = ObjectStoreHandler::new(None);

    let callback = Arc::new(PrintCallback);

    match handler
        .list_objects(
            &ParsedUri::from_uri(&uri, true),
            config,
            None, // functions as "*", prints all columns
            recursive,
            Some(max_files),
            &filter,
            Some(callback),
        )
        .await
    {
        Ok(_) => {
            debug!("List objects executed successfully with no return value.");
        }
        Err(InternalError::NoBucketInUri(_)) => {
            error!("Error: No bucket in URI");
            std::process::exit(1);
        }
        Err(err) => {
            error!("Error listing objects: {}", err);
            std::process::exit(1);
        }
    }
}

fn prepare_handle_ls_arguments(
    ls_matches: &clap::ArgMatches,
) -> (String, bool, u32, Option<FileObjectFilter>) {
    let recursive = *ls_matches.get_one::<bool>("recursive").unwrap_or(&false);
    let uri = ls_matches.get_one::<String>("uri").unwrap().to_string();

    // uri should start with a scheme, if not add default
    let uri = if uri.contains("://") {
        uri
    } else {
        format!("localfs://{}", uri)
    };

    let filter_name = ls_matches
        .get_one::<String>("name")
        .map(ToString::to_string);
    let filter_size = ls_matches
        .get_one::<String>("size")
        .map(ToString::to_string);
    let filter_mtime = ls_matches
        .get_one::<String>("mtime")
        .map(ToString::to_string);

    let filter = match (&filter_name, &filter_size, &filter_mtime) {
        (None, None, None) => None,
        _ => {
            let filter_result = FileObjectFilter::new_with_single_condition(
                filter_name.as_deref(),
                filter_size.as_deref(),
                filter_mtime.as_deref(),
            );
            match filter_result {
                Ok(filter) => Some(filter),
                Err(err) => {
                    error!("Error creating filter: {}", err);
                    std::process::exit(1);
                }
            }
        }
    };

    let max_files = ls_matches
        .get_one::<String>("max_files")
        .unwrap()
        .parse::<u32>()
        .expect("Invalid value for max_files");

    (uri, recursive, max_files, filter)
}

// Callback to print each row to the console
struct PrintCallback;
impl TableCallback for PrintCallback {
    fn on_row_add(&self, row: &mut TableRow) {
        row.print();
    }
}
