use std::fs;
use std::path::Path;
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
    let no_recursive = *ls_matches.get_one::<bool>("no_recursive").unwrap_or(&false);
    let show_hidden =
        *ls_matches.get_one::<bool>("show_hidden").unwrap_or(&false);

    let no_gitignore =
        *ls_matches.get_one::<bool>("no_gitignore").unwrap_or(&false);

    // Fetching a vector of specified ignore files, if any
    let other_ignore_files = ls_matches
        .get_many::<String>("other_ignore_files")
        .map_or(Vec::new(), |vals| {
            vals.map(String::from).collect::<Vec<_>>()
        });

    // Check if '.gitignore' is already included in the user-specified files
    let gitignore_included =
        other_ignore_files.iter().any(|file| file == ".gitignore");

    // Include '.gitignore' by default unless no_gitignore is set or it's already included
    let ignore_files = if !no_gitignore && !gitignore_included {
        vec![".gitignore".to_string()]
            .into_iter()
            .chain(other_ignore_files.into_iter())
            .collect()
    } else {
        other_ignore_files
    };

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

    let (root_path, ignore_contents) = get_ignore_contents(&uri, &ignore_files);

    let filter =
        match (&filter_name, &filter_size, &filter_mtime, &ignore_contents) {
            (None, None, None, None) => None,
            _ => {
                let filter_result = FileObjectFilter::new_with_single_condition(
                    filter_name.as_deref(),
                    filter_size.as_deref(),
                    filter_mtime.as_deref(),
                    root_path, // root_path - only used when ignore_contents is Some
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

fn get_ignore_contents(
    uri: &str,
    ignore_files: &Vec<String>,
) -> (Option<&'static Path>, Option<String>) {
    // Currently only localfs is supported
    if uri.starts_with("localfs://") {
        let mut contents = String::new();
        for ignore_file in ignore_files {
            let gitignore_path = Path::new(ignore_file);
            if gitignore_path.exists() {
                match fs::read_to_string(gitignore_path) {
                    Ok(content) => contents.push_str(&content),
                    Err(error) => {
                        log::error!("Error reading ignore file: {}", error)
                    }
                }
            }
        }
        if !contents.is_empty() {
            let root_path = Path::new(".");
            return (Some(root_path), Some(contents)); // Successfully aggregated the contents
        }
    }
    (None, None)
}
