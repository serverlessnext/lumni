use std::collections::HashMap;
use std::fs::File;

use tempfile::tempdir;
use xlatti::{EnvironmentConfig, ListObjectsResult, ObjectStoreHandler};

#[tokio::test]
async fn test_list_objects() {
    // Create a temporary directory.
    let temp_dir = tempdir().expect("Failed to create temp dir");
    let temp_dir_path = temp_dir.path().to_path_buf();

    // Create sample files in the temporary directory.
    let file_path1 = temp_dir_path.join("file1.txt");
    let file_path2 = temp_dir_path.join("file2.txt");
    File::create(&file_path1).unwrap();
    File::create(&file_path2).unwrap();

    let settings = HashMap::new();
    let config = EnvironmentConfig::new(settings);
    let handler = ObjectStoreHandler::new(None);
    let uri = format!("localfs://{}", temp_dir_path.display());
    let recursive = false;
    let max_files = None;
    let filter = None;
    let callback = None;

    let result = handler
        .list_objects(&uri, &config, recursive, max_files, &filter, callback)
        .await
        .unwrap();

    let file_objects = match result {
        Some(ListObjectsResult::FileObjects(fo)) => fo,
        _ => panic!("Unexpected result type"),
    };

    let filenames: Vec<String> = file_objects
        .iter()
        .map(|fo| fo.name())
        .map(|name| name.to_string())
        .collect();

    let file_path1_str = file_path1.to_string_lossy().into_owned();
    let file_path2_str = file_path2.to_string_lossy().into_owned();

    assert!(filenames.contains(&file_path1_str));
    assert!(filenames.contains(&file_path2_str));
}
