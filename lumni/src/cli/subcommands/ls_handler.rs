use std::sync::Arc;

use log::{debug, error};
use lumni::{
    EnvironmentConfig, FileObjectFilter, IgnoreContents, InternalError,
    ObjectStoreHandler, ParsedUri, TableCallback, TableRow,
};

pub async fn handle_ls(
    ls_matches: &clap::ArgMatches,
    config: &mut EnvironmentConfig,
) {
    let (uri, skip_hidden, recursive, max_files, filter) =
        prepare_handle_ls_arguments(ls_matches);

    let handler = ObjectStoreHandler::new(None);

    let callback = Arc::new(PrintCallback);
    match handler
        .list_objects(
            &ParsedUri::from_uri(&uri, true),
            config,
            None, // functions as "*", prints all columns
            skip_hidden,
            recursive,
            max_files,
            filter,
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
) -> (String, bool, bool, Option<u32>, Option<FileObjectFilter>) {
    let no_recursive =
        *ls_matches.get_one::<bool>("no_recursive").unwrap_or(&false);
    let show_hidden =
        *ls_matches.get_one::<bool>("show_hidden").unwrap_or(&false);
    let no_gitignore =
        *ls_matches.get_one::<bool>("no_gitignore").unwrap_or(&false);
    let other_ignore_files = ls_matches
        .get_many::<String>("other_ignore_files")
        .map_or(Vec::new(), |vals| {
            vals.map(String::from).collect::<Vec<_>>()
        });

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

    let ignore_contents =
        Some(IgnoreContents::new(other_ignore_files, !no_gitignore));

    let filter =
        match (&filter_name, &filter_size, &filter_mtime, &ignore_contents) {
            (None, None, None, None) => None,
            _ => {
                let filter_result = FileObjectFilter::new_with_single_condition(
                    filter_name.as_deref(),
                    filter_size.as_deref(),
                    filter_mtime.as_deref(),
                    ignore_contents,
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

    let max_files: Option<u32> =
        ls_matches.get_one::<String>("max_files").map(|max_files| {
            max_files.parse::<u32>().unwrap_or_else(|err| {
                error!("Error parsing max_files: {}", err);
                std::process::exit(1);
            })
        });

    (uri, !show_hidden, !no_recursive, max_files, filter)
}

// Callback to print each row to the console
struct PrintCallback;
impl TableCallback for PrintCallback {
    fn on_row_add(&self, row: &mut TableRow) {
        row.print();
    }
}
