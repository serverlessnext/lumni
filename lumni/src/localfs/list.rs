use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crossbeam_channel::{bounded, Sender};
use rayon::prelude::*;

use crate::table::{FileObjectTable, TableColumnValue};
use crate::{FileObject, FileObjectFilter, FileType, InternalError};

pub async fn list_files(
    path: &Path,
    selected_columns: &Option<Vec<&str>>,
    max_keys: Option<u32>,
    skip_hidden: bool,
    recursive: bool,
    filter: &Option<FileObjectFilter>,
    table: &mut FileObjectTable,
) -> Result<(), InternalError> {
    if let Some(columns) = selected_columns {
        println!("Selected columns: {:?}", columns);
    }

    let max_count = max_keys.map(|m| m as usize).unwrap_or(usize::MAX);
    let count = Arc::new(AtomicUsize::new(0));
    let (sender, receiver) = bounded(500);

    let path_buf = path.to_path_buf(); // Clone the path

    // Spawn a thread to process entries
    let count_clone = count.clone();
    let filter_clone = filter.clone();
    std::thread::spawn(move || {
        process_directory(
            &path_buf,
            skip_hidden,
            recursive,
            &filter_clone,
            &count_clone,
            max_count,
            &sender,
        );
    });

    let mut rows: Vec<_> = receiver
        .into_iter()
        .filter_map(|entry| {
            let result = process_entry(&entry, filter, selected_columns);
            if result.is_some() {
                // Increment only if process_entry returns an entry
                count.fetch_add(1, Ordering::Relaxed);
            }
            result
        })
        .take(max_count)
        .collect();

    // In non-recursive mode, sort the rows so that directories come first
    // skipped in recursive mode because directories are not shown separately
    if !recursive {
        rows.sort_by(|a, b| {
            let a_is_dir = a.get("type")
                .map(|t| matches!(t, TableColumnValue::Uint8Column(v) if *v == FileType::Directory.to_u8()))
                .unwrap_or(false);
            let b_is_dir = b.get("type")
                .map(|t| matches!(t, TableColumnValue::Uint8Column(v) if *v == FileType::Directory.to_u8()))
                .unwrap_or(false);

            if a_is_dir && !b_is_dir {
                std::cmp::Ordering::Less
            } else if !a_is_dir && b_is_dir {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        });
    }

    // Batch insert all rows at once
    if !rows.is_empty() {
        table.add_rows(rows).await?;
    }
    Ok(())
}

fn process_directory(
    path: &Path,
    skip_hidden: bool,
    recursive: bool,
    filter: &Option<FileObjectFilter>,
    count: &AtomicUsize,
    max_count: usize,
    sender: &Sender<fs::DirEntry>,
) {
    if count.load(Ordering::Relaxed) >= max_count {
        return;
    }

    if let Ok(entries) = fs::read_dir(path) {
        entries.par_bridge().for_each(|entry| {
            if count.load(Ordering::Relaxed) >= max_count {
                return;
            }

            if let Ok(entry) = entry {
                // Check for hidden files if skip_hidden is true
                if skip_hidden
                    && entry.file_name().to_string_lossy().starts_with('.')
                {
                    return;
                }

                if let Some(filter) = filter {
                    let path_name = entry.path();
                    if filter.ignore_matches(&path_name) {
                        return;
                    }
                }

                if let Ok(file_type) = entry.file_type() {
                    match file_type {
                        t if t.is_dir() => {
                            if recursive {
                                process_directory(
                                    &entry.path(),
                                    skip_hidden,
                                    recursive,
                                    filter,
                                    count,
                                    max_count,
                                    sender,
                                );
                            };
                        }
                        t if t.is_file() => {}
                        _ => return, // Ignore other file types
                    }
                    let _ = sender.send(entry);
                }
            }
        });
    }
}

fn process_entry(
    entry: &fs::DirEntry,
    filter: &Option<FileObjectFilter>,
    selected_columns: &Option<Vec<&str>>,
) -> Option<HashMap<String, TableColumnValue>> {
    let metadata = entry.metadata().ok()?;

    if metadata.is_file() {
        handle_file(entry, filter, selected_columns)
    } else if metadata.is_dir() {
        // Include directory if there's no filter, or if the filter includes directories
        if filter.as_ref().map_or(true, |f| f.include_directories) {
            handle_directory(entry, selected_columns)
        } else {
            None
        }
    } else {
        // ignore any other file types (symlinks, pipes, sockets, etc.)
        None
    }
}

fn handle_directory(
    entry: &fs::DirEntry,
    selected_columns: &Option<Vec<&str>>,
) -> Option<HashMap<String, TableColumnValue>> {
    let dir_name = entry
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_string()
        + "/";
    let mut dir_row_data = HashMap::new();
    if selected_columns
        .as_ref()
        .map_or(true, |cols| cols.contains(&"name"))
    {
        dir_row_data.insert(
            "name".to_string(),
            TableColumnValue::StringColumn(dir_name),
        );
    }
    if selected_columns
        .as_ref()
        .map_or(true, |cols| cols.contains(&"size"))
    {
        dir_row_data
            .insert("size".to_string(), TableColumnValue::Uint64Column(0));
    }
    if selected_columns
        .as_ref()
        .map_or(true, |cols| cols.contains(&"modified"))
    {
        dir_row_data.insert(
            "modified".to_string(),
            TableColumnValue::OptionalInt64Column(None),
        );
    }
    if selected_columns
        .as_ref()
        .map_or(true, |cols| cols.contains(&"type"))
    {
        dir_row_data.insert(
            "type".to_string(),
            TableColumnValue::Uint8Column(FileType::Directory.to_u8()),
        );
    }

    if dir_row_data.is_empty() {
        None
    } else {
        Some(dir_row_data)
    }
}

fn handle_file(
    entry: &fs::DirEntry,
    filter: &Option<FileObjectFilter>,
    selected_columns: &Option<Vec<&str>>,
) -> Option<HashMap<String, TableColumnValue>> {
    let metadata = entry.metadata().ok()?;

    let file_name = entry.path().to_string_lossy().to_string();
    let file_size = metadata.len();
    let modified = metadata.modified().ok().map(|mtime| {
        mtime
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_secs())
            .unwrap_or(0) as i64
    });
    // Check if the file_object satisfies the filter conditions
    let file_object = FileObject::new(
        file_name.clone(),
        file_size,
        FileType::RegularFile,
        modified,
        None,
    );
    if let Some(ref filter) = filter {
        if !filter.condition_matches(&file_object) {
            return None;
        }
    }
    let mut row_data = HashMap::new();
    if selected_columns
        .as_ref()
        .map_or(true, |cols| cols.contains(&"name"))
    {
        row_data.insert(
            "name".to_string(),
            TableColumnValue::StringColumn(file_name),
        );
    }
    if selected_columns
        .as_ref()
        .map_or(true, |cols| cols.contains(&"size"))
    {
        row_data.insert(
            "size".to_string(),
            TableColumnValue::Uint64Column(file_size),
        );
    }
    if selected_columns
        .as_ref()
        .map_or(true, |cols| cols.contains(&"modified"))
        && modified.is_some()
    {
        row_data.insert(
            "modified".to_string(),
            TableColumnValue::OptionalInt64Column(modified),
        );
    }
    if selected_columns
        .as_ref()
        .map_or(true, |cols| cols.contains(&"type"))
    {
        row_data.insert(
            "type".to_string(),
            TableColumnValue::Uint8Column(FileType::RegularFile.to_u8()),
        );
    }

    if row_data.is_empty() {
        None
    } else {
        Some(row_data)
    }
}
