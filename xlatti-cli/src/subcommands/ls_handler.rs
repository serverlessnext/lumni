use std::sync::Arc;

use std::collections::HashMap;
use log::info;
use xlatti::{
    EnvironmentConfig,
    FileObjectFilter, LakestreamError, ObjectStoreHandler,
    TableCallback, TableColumnValue,
};

pub async fn handle_ls(
    ls_matches: &clap::ArgMatches,
    config: &mut EnvironmentConfig,
) {
    let (uri, recursive, max_files, filter) =
        prepare_handle_ls_arguments(ls_matches);

    let handler = ObjectStoreHandler::new(None);

    let callback = Arc::new(FileObjectCallback);

    match handler
        .list_objects(
            &uri,
            config,
            recursive,
            Some(max_files),
            &filter,
            Some(callback),
        )
        .await
    {
        //Ok(Some(list_objects_result)) => {
        //    //handle_list_objects_result(list_objects_result).await;
        //}
        Ok(_) => {
            println!("Done");
        }
        Err(LakestreamError::NoBucketInUri(_)) => {
            // if uri ends with "/", try to list buckets instead
            if uri.ends_with('/') {
                handle_list_buckets(&uri, config).await;
            } else {
                eprintln!("Error: No bucket found at: {}", uri);
            }
        }
        Err(err) => {
            eprintln!("Error: {:?}", err);
        }
    }
}

//pub async fn handle_list_objects_result(
//    list_objects_result: ListObjectsResult,
//) {
//    match list_objects_result {
//        ListObjectsResult::RowItems(items) => {
//            // Print buckets to stdout
//            info!("Found {} items:", items.len());
//            for item in items {
//                println!("{}", item.name());
//            }
//        }
//        ListObjectsResult::FileObjects(file_objects) => {
//            // Print file objects to stdout
//            info!("Found {} file objects:", file_objects.len());
//            for fo in file_objects {
//                println!("{}", fo.println_path());
//            }
//        }
//    }
//}

async fn handle_list_buckets(uri: &str, config: &EnvironmentConfig) {
    log::info!("Calling list_buckets");
    let handler = ObjectStoreHandler::new(None);
    let callback = Arc::new(ObjectStoreCallback);
    match handler.list_buckets(uri, config, Some(callback)).await {
        Ok(_) => {}
        Err(err) => {
            eprintln!("Error: {:?}", err);
        }
    }
}

fn prepare_handle_ls_arguments(
    ls_matches: &clap::ArgMatches,
) -> (String, bool, u32, Option<FileObjectFilter>) {
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

    (uri, recursive, max_files, filter)
}

// Callback to print buckets
struct ObjectStoreCallback;
impl TableCallback for ObjectStoreCallback {
    fn on_row_add(&self, row: &mut HashMap<String, TableColumnValue>) {
        let uri = row.get("uri").unwrap().to_string();
        println!("{}", uri);
    }
}

// Callback to print file objects
struct FileObjectCallback;
impl TableCallback for FileObjectCallback {
    fn on_row_add(&self, row: &mut HashMap<String, TableColumnValue>) {
        let name = row.get("name").unwrap().to_string();
        let size = row.get("size").unwrap().to_string();
        println!("{} - {}", size, name);
    }
}
