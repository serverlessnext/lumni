
use std::collections::HashMap;

use lakestream::{
    Config, FileObject, FileObjectFilter, ListObjectsResult, ObjectStoreHandler, CallbackWrapper,
};

pub async fn handle_ls(ls_matches: &clap::ArgMatches, region: Option<String>) {
    let (uri, config, recursive, max_files, filter) =
        prepare_handle_ls_arguments(ls_matches, region);

    let handler = ObjectStoreHandler::new(vec![config.clone()]);

    // print via callback function (sync or async supported)
    // let callback = Some(CallbackWrapper::create_sync(print_file_objects_callback));
    let callback = Some(CallbackWrapper::create_async(print_file_objects_callback_async));

    // get results as a return value instead of a callback
    // let callback = None;

    match handler.list_objects_with_callback(
        uri,
        config,
        recursive,
        Some(max_files),
        &filter,
        callback,
    )
    .await
    {
        Ok(Some(list_objects_result)) => {
            match list_objects_result {
                ListObjectsResult::Buckets(buckets) => {
                   // Print buckets to stdout
                    println!("Found {} buckets:", buckets.len());
                    for bucket in buckets {
                        println!("{}", bucket.name());
                    }
                }
                ListObjectsResult::FileObjects(file_objects) => {
                   // Print file objects to stdout
                    println!("Found {} file objects:", file_objects.len());
                    for fo in file_objects {
                        let full_path = true;
                        println!("{}", fo.printable(full_path));
                    }
                }
            }
        }
        Ok(None) => {
            // If you don't need to handle this case specifically, you can just use `println!("Done.");`
            println!("Done.");
        }
        Err(err) => {
            // Handle the error case
            eprintln!("Error: {:?}", err);
        }
    }
}

fn prepare_handle_ls_arguments(
    ls_matches: &clap::ArgMatches,
    region: Option<String>,
) -> (String, Config, bool, u32, Option<FileObjectFilter>) {
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

    let mut config_hashmap = HashMap::new();
    if let Some(region) = region {
        config_hashmap.insert("region".to_string(), region);
    }

    // Create a Config instance
    let config = Config {
        settings: config_hashmap,
    };

    (uri, config, recursive, max_files, filter)
}

// keep this for reference -- async is default now
fn print_file_objects_callback(file_objects: &[FileObject]) {
    let full_path = true;
    // println!("Found {} file objects:", file_objects.len());
    for fo in file_objects {
        println!("{}", fo.printable(full_path));
    }
}

async fn print_file_objects_callback_async(file_objects: Vec<FileObject>) {
    let full_path = true;
    // println!("Found {} file objects:", file_objects.len());
    for fo in &file_objects {
        println!("{}", fo.printable(full_path));
    }
}

