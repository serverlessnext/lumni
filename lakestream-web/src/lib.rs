use std::collections::HashMap;

use ::lakestream::{FileObjectFilter, ListObjectsResult, ObjectStoreHandler};
use leptos::*;

#[component]
pub fn App(cx: Scope) -> impl IntoView {
    let (count, set_count) = create_signal(cx, 0);

    let async_data = create_resource(cx, count, |count| async move {
        list_objects_demo(count).await
    });

    let stable = create_resource(
        cx,
        || (),
        |_| async move { list_objects_demo(1).await },
    );

    let async_result = move || {
        log!("async_result");
        async_data
            .read(cx)
            .map(|files| format!("Files: {:?}", files))
            .unwrap_or_else(|| "Loading...".into())
    };

    let loading = async_data.loading();
    let is_loading = move || if loading() { "Loading..." } else { "Idle." };

    view! { cx,
        <button
            on:click=move |_| {
                set_count.update(|n| *n += 1);
            }
        >
            "Refresh me"
        </button>
        <p>
            <code>"stable"</code>": " {move || stable.read(cx).map(|files| format!("{:?}", files))}
        </p>
        <p>
            <code>"count"</code>": " {count}
        </p>
        <p>
            <code>"async_value"</code>": "
            {async_result}
            <br/>
            {is_loading}
        </p>
    }
}

async fn list_objects_demo(_count: i32) -> Vec<String> {
    let mut config = HashMap::new();
    let recursive = false;
    let max_files = Some(20);
    let filter: Option<FileObjectFilter> = None;

    // TODO: get from user input
    let uri = "s3://".to_string();
    config.insert(
        "AWS_ACCESS_KEY_ID".to_string(),
        "__INSERT_ACCESS_KEY__".to_string(),
    );
    config.insert(
        "AWS_SECRET_ACCESS_KEY".to_string(),
        "__INSERT_SECRET_ACCESS_KEY__".to_string(),
    );

    let result = ObjectStoreHandler::list_objects(
        uri, config, recursive, max_files, &filter,
    )
    .await;

    let files: Vec<String> = match result {
        Ok(ListObjectsResult::FileObjects(file_objects)) => {
            let file_names = file_objects
                .into_iter()
                .map(|fo| fo.name().to_owned())
                .collect::<Vec<_>>();
            file_names
        }
        Ok(ListObjectsResult::Buckets(buckets)) => {
            // note - CORS does not work on Bucket List
            let bucket_names = buckets
                .into_iter()
                .map(|bucket| bucket.name().to_owned())
                .collect::<Vec<_>>();
            bucket_names
        }
        Err(err) => {
            log!("Error: {:?}", err);
            vec![]
        }
    };
    files
}
