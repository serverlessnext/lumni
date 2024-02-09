use log::info;
use xlatti::{
    CallbackItem, CallbackWrapper, EnvironmentConfig, FileObjectFilter,
    LakestreamError, ListObjectsResult, ObjectStoreHandler,
};

pub async fn handle_ls(
    ls_matches: &clap::ArgMatches,
    config: &mut EnvironmentConfig,
) {
    let (uri, recursive, max_files, filter) =
        prepare_handle_ls_arguments(ls_matches);

    let handler = ObjectStoreHandler::new(None);

    let callback =
        Some(CallbackWrapper::create_async(print_callback_items_async));

    match handler
        .list_objects(
            &uri,
            config,
            recursive,
            Some(max_files),
            &filter,
            callback,
        )
        .await
    {
        Ok(Some(list_objects_result)) => {
            handle_list_objects_result(list_objects_result).await;
        }
        Ok(None) => {
            println!("Done");
        }
        Err(LakestreamError::NoBucketInUri(_)) => {
            handle_list_buckets(&uri, config).await;
        }
        Err(err) => {
            eprintln!("Error: {:?}", err);
        }
    }
}

async fn handle_list_objects_result(list_objects_result: ListObjectsResult) {
    match list_objects_result {
        ListObjectsResult::Buckets(buckets) => {
            // Print buckets to stdout
            info!("Found {} buckets:", buckets.len());
            for bucket in buckets {
                println!("{}", bucket.name());
            }
        }
        ListObjectsResult::FileObjects(file_objects) => {
            // Print file objects to stdout
            info!("Found {} file objects:", file_objects.len());
            for fo in file_objects {
                println!("{}", fo.println_path());
            }
        }
    }
}

async fn handle_list_buckets(uri: &str, config: &EnvironmentConfig) {
    log::info!("Calling list_buckets");
    let handler = ObjectStoreHandler::new(None);
    let callback =
        Some(CallbackWrapper::create_async(print_callback_items_async));
    match handler.list_buckets(uri, config, callback).await {
        Ok(Some(list_objects_result)) => {
            handle_list_objects_result(list_objects_result).await;
        }
        Ok(None) => {
            log::info!("Done");
        }
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

async fn print_callback_items_async<T: CallbackItem>(items: Vec<T>) {
    info!("Found {} items:", items.len());
    for item in &items {
        println!("{}", item.println_path());
    }
}
