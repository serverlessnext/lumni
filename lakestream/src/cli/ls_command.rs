use std::collections::HashMap;

use crate::{FileObjectFilter, ListObjectsResult, ObjectStoreHandler};

pub async fn handle_ls(ls_matches: &clap::ArgMatches, region: Option<String>) {
    let (uri, config, recursive, max_files, filter) =
        prepare_handle_ls_arguments(ls_matches, region);

    println!("Listing objects at {}", uri);
    match ObjectStoreHandler::list_objects(
        uri,
        config,
        recursive,
        Some(max_files),
        &filter,
    )
    .await
    {
        Ok(ListObjectsResult::FileObjects(file_objects)) => {
            // Print file objects to stdout
            println!("Found {} file objects:", file_objects.len());
            for fo in file_objects {
                let full_path = true;
                println!("{}", fo.printable(full_path));
            }
        }
        Ok(ListObjectsResult::Buckets(buckets)) => {
            // Print buckets to stdout
            println!("Found {} buckets:", buckets.len());
            for bucket in buckets {
                println!("{}", bucket.name());
            }
        }
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        }
    }
}

fn prepare_handle_ls_arguments(
    ls_matches: &clap::ArgMatches,
    region: Option<String>,
) -> (
    String,
    HashMap<String, String>,
    bool,
    u32,
    Option<FileObjectFilter>,
) {
    let recursive = *ls_matches.get_one::<bool>("recursive").unwrap_or(&false);
    let uri = ls_matches.get_one::<String>("uri").unwrap().to_string();

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
            let filter_result = FileObjectFilter::new(
                filter_name.as_deref(),
                filter_size.as_deref(),
                filter_mtime.as_deref(),
            );
            match filter_result {
                Ok(filter) => Some(filter),
                Err(err) => {
                    eprintln!("Error: {}", err);
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

    let mut config = HashMap::new();
    if let Some(region) = region {
        config.insert("region".to_string(), region);
    }

    (uri, config, recursive, max_files, filter)
}
