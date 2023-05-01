use std::fs;
use std::path::Path;

use crate::{FileObject, FileObjectFilter, FileObjectVec};

pub async fn list_files(
    path: &Path,
    max_keys: Option<u32>,
    recursive: bool,
    filter: &Option<FileObjectFilter>,
    file_objects: &mut FileObjectVec,
) {
    list_files_next(path, max_keys, recursive, filter, file_objects).await;
}

async fn list_files_next(
    path: &Path,
    max_keys: Option<u32>,
    recursive: bool,
    filter: &Option<FileObjectFilter>,
    file_objects: &mut FileObjectVec,
) {
    let mut directory_stack = vec![path.to_owned()];

    while let Some(current_path) = directory_stack.pop() {
        let mut temp_file_objects = Vec::new();

        if let Ok(entries) = fs::read_dir(&current_path) {
            for entry in entries.flatten() {
                if let Some(max_keys) = max_keys {
                    if file_objects.len() >= max_keys as usize {
                        break;
                    }
                }

                let metadata = match entry.metadata() {
                    Ok(md) => md,
                    Err(_) => continue,
                };

                if metadata.is_file() {
                    let file_object = handle_file(&entry, filter);
                    if let Some(file_object) = file_object {
                        temp_file_objects.push(file_object);
                    }
                } else if metadata.is_dir() {
                    let dir_name = entry.path().to_string_lossy().to_string();

                    // Only add directory object when no filter is provided
                    if filter.is_none() {
                        let dir_object =
                            FileObject::new(dir_name, 0, None, None);
                        temp_file_objects.push(dir_object);
                    }

                    if recursive {
                        directory_stack.push(entry.path());
                    }
                }
            }
        }
        file_objects.extend_async(temp_file_objects).await;
    }
}

fn handle_file(
    entry: &fs::DirEntry,
    filter: &Option<FileObjectFilter>,
) -> Option<FileObject> {
    let metadata = match entry.metadata() {
        Ok(md) => md,
        Err(_) => return None,
    };

    let file_name = entry.path().to_string_lossy().to_string();
    let file_size = metadata.len();
    let modified = metadata.modified().ok().map(|mtime| {
        mtime
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0)
    });

    let file_object = FileObject::new(file_name, file_size, modified, None);

    // Check if the file_object satisfies the filter conditions
    if let Some(ref filter) = filter {
        if !filter.matches(&file_object) {
            return None;
        }
    }

    Some(file_object)
}
