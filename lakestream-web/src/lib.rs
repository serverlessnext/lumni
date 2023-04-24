use leptos::*;

use std::collections::HashMap;

// start with :: to ensure local crate is used
use ::lakestream::{
    ListObjectsResult, ObjectStoreHandler,
};


#[component]
pub fn App(cx: Scope) -> impl IntoView {

    let files = list_files();
    view! { cx,
        <StaticList files={files} />
    }
}

#[component]
fn StaticList(
    cx: Scope,
    files: Vec<String>,
) -> impl IntoView {
    // iterate through the files vector and create a list of file names
    let file_objects = files
        .into_iter()
        .enumerate()
        .map(|(index, file)| {
            view! { cx,
                <li>
                    {format!("{}: {}", index, file)}
                </li>
            }
        })
        .collect::<Vec<_>>();

    view! { cx,
        <ul>{file_objects}</ul>
    }
}

fn list_files() -> Vec<String> {
    let uri = ".".to_string();
    let config = HashMap::new();
    let recursive = false;
    let max_files = 10;
    let filter = None;

    let files: Vec<String> = match ObjectStoreHandler::list_objects(
        uri,
        config.clone(),
        recursive,
        Some(max_files),
        &filter,
    ) {
        ListObjectsResult::FileObjects(file_objects) => {
            log!("FileObjects: {:?}", file_objects);
            // Generate a dummy list of Vec<String> of files
            (0..5).map(|i| format!("file-{}", i)).collect()
        }
        ListObjectsResult::Buckets(buckets) => {
            log!("Buckets: {:?}", buckets.iter().map(|b| b.name()).collect::<Vec<_>>());
            // Generate a dummy list of Vec<String> of files
            (0..5).map(|i| format!("bucket-{}", i)).collect()
        }
    };
    files
}

