use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use crossbeam_channel::{bounded, Sender};
use rayon::prelude::*;

use crate::table::{FileObjectTable, TableColumnValue};
use crate::{FileObject, FileObjectFilter};

pub async fn list_files(
    path: &Path,
    selected_columns: &Option<Vec<&str>>,
    max_keys: Option<u32>,
    recursive: bool,
    filter: &Option<FileObjectFilter>,
    table: &mut FileObjectTable,
) {
    println!("Selected columns: {:?}", selected_columns);

    let max_count = max_keys.map(|m| m as usize).unwrap_or(usize::MAX);
    let count = AtomicUsize::new(0);
    let (sender, receiver) = bounded(500);

    let path_buf = path.to_path_buf(); // Clone the path

    // Spawn a thread to process entries
    std::thread::spawn(move || {
        process_directory(&path_buf, recursive, &count, max_count, &sender);
    });

    let rows: Vec<_> = receiver
        .into_iter()
        .filter_map(|entry| process_entry(&entry, filter, selected_columns))
        .take(max_count)
        .collect();

    // Batch insert all rows at once
    if !rows.is_empty() {
        let _ = table.add_rows(rows).await;
    }
}

fn process_directory(
    path: &Path,
    recursive: bool,
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
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() && recursive {
                        process_directory(
                            &entry.path(),
                            recursive,
                            count,
                            max_count,
                            sender,
                        );
                    }
                    if file_type.is_file() || file_type.is_dir() {
                        count.fetch_add(1, Ordering::Relaxed);
                        let _ = sender.send(entry);
                    }
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
    } else if metadata.is_dir() && filter.is_none() {
        handle_directory(entry, selected_columns)
    } else {
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
        .to_string();
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
            TableColumnValue::OptionalUint64Column(None),
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
            .unwrap_or(0)
    });

    // Check if the file_object satisfies the filter conditions
    let file_object =
        FileObject::new(file_name.clone(), file_size, modified, None);
    if let Some(ref filter) = filter {
        if !filter.matches(&file_object) {
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
            TableColumnValue::OptionalUint64Column(modified),
        );
    }

    if row_data.is_empty() {
        None
    } else {
        Some(row_data)
    }
}
