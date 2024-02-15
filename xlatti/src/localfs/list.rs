use std::fs;
use std::path::Path;

use super::bucket::{FileSystem, LocalFileSystem};
use crate::{FileObject, FileObjectFilter, RowItem, RowItemVec, RowType};

pub async fn list_files(
    path: &Path,
    max_keys: Option<u32>,
    recursive: bool,
    filter: &Option<FileObjectFilter>,
    file_objects: &mut RowItemVec,
) {
    list_files_next(path, max_keys, recursive, filter, file_objects).await;
}

async fn list_files_next(
    path: &Path,
    max_keys: Option<u32>,
    recursive: bool,
    filter: &Option<FileObjectFilter>,
    file_objects: &mut RowItemVec,
) {
    let fs = &LocalFileSystem;
    let mut directory_stack = vec![path.to_owned()];

    while let Some(current_path) = directory_stack.pop() {
        let mut temp_row_items = Vec::new();

        if let Ok(entries) = fs.read_dir(&current_path) {
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
                        let row_item =
                            RowItem::new(RowType::FileObject(file_object));
                        temp_row_items.push(row_item);
                    }
                } else if metadata.is_dir() {
                    let dir_name = entry.path().to_string_lossy().to_string();

                    // Only add directory object when no filter is provided
                    if filter.is_none() {
                        let dir_object =
                            FileObject::new(dir_name, 0, None, None);
                        let row_item =
                            RowItem::new(RowType::FileObject(dir_object));
                        temp_row_items.push(row_item);
                    }

                    if recursive {
                        directory_stack.push(entry.path());
                    }
                }
            }
        }
        file_objects.extend_async(temp_row_items).await;
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
