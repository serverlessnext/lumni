use std::fs;
use std::path::Path;

use crate::{FileObject, FileObjectFilter};

pub fn list_files(
    path: &Path,
    max_keys: Option<u32>,
    recursive: bool,
    filter: &Option<FileObjectFilter>,
) -> Vec<FileObject> {
    let mut file_objects = Vec::new();
    list_files_next(path, max_keys, recursive, filter, &mut file_objects);
    file_objects
}

fn handle_file(
    entry: &fs::DirEntry,
    filter: &Option<FileObjectFilter>,
    file_objects: &mut Vec<FileObject>,
) -> u32 {
    let metadata = match entry.metadata() {
        Ok(md) => md,
        Err(_) => return 0,
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
            return 0;
        }
    }

    file_objects.push(file_object);
    1
}

fn handle_directory(
    entry: &fs::DirEntry,
    max_keys: Option<u32>,
    recursive: bool,
    filter: &Option<FileObjectFilter>,
    file_objects: &mut Vec<FileObject>,
) -> u32 {
    let dir_name = entry.path().to_string_lossy().to_string();

    // Only add directory object when no filter is provided
    if filter.is_none() {
        let dir_object = FileObject::new(dir_name, 0, None, None);
        file_objects.push(dir_object);
    }

    if !recursive {
        return 1;
    }

    list_files_next(
        entry.path().as_path(),
        max_keys,
        recursive,
        filter,
        file_objects,
    ) + 1
}

fn list_files_next(
    path: &Path,
    max_keys: Option<u32>,
    recursive: bool,
    filter: &Option<FileObjectFilter>,
    file_objects: &mut Vec<FileObject>,
) -> u32 {
    let mut count = 0;

    if let Ok(entries) = fs::read_dir(path) {
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
                count += handle_file(&entry, filter, file_objects);
            } else if metadata.is_dir() {
                count += handle_directory(
                    &entry,
                    max_keys,
                    recursive,
                    filter,
                    file_objects,
                );
            }
        }
    }
    count
}
