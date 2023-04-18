use std::collections::HashMap;
use std::fs;
use std::path::Path;

use crate::base::interfaces::ObjectStoreTrait;
use crate::FileObject;

pub struct LocalFs {
    name: String,
    #[allow(dead_code)]
    config: HashMap<String, String>,
}

impl LocalFs {
    pub fn new(
        name: &str,
        config: HashMap<String, String>,
    ) -> Result<LocalFs, &'static str> {
        Ok(LocalFs {
            name: name.to_string(),
            config,
        })
    }
}

impl ObjectStoreTrait for LocalFs {
    fn name(&self) -> &str {
        &self.name
    }

    fn config(&self) -> &HashMap<String, String> {
        &self.config
    }

    fn list_files(
        &self,
        prefix: Option<&str>,
        recursive: bool,
        max_keys: Option<u32>,
    ) -> Vec<FileObject> {
        let path = match prefix {
            Some(prefix) => Path::new(&self.name).join(prefix),
            None => Path::new(&self.name).to_path_buf(),
        };
        // TODO: enable recursive listing
        // list_files_in_directory() appears to work correctly on recursive
        // but should still make this parameter work for both S3Bucket and LocalFs
        // also, the print to stdout in FileObject Impl needs to be fixed
        list_files_in_directory(&path, max_keys, recursive)
    }
}

fn list_files_in_directory(
    path: &Path,
    max_keys: Option<u32>,
    recursive: bool,
) -> Vec<FileObject> {
    let mut file_objects = Vec::new();
    let mut count = 0;

    fn list_files_recursive(
        path: &Path,
        max_keys: Option<u32>,
        recursive: bool,
        count: &mut u32,
        file_objects: &mut Vec<FileObject>,
    ) {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Some(max_keys) = max_keys {
                        if *count >= max_keys {
                            break;
                        }
                    }

                    if let Ok(metadata) = entry.metadata() {
                        if metadata.is_file() {
                            let file_name =
                                entry.path().to_string_lossy().to_string();
                            let file_size = metadata.len();
                            // Get the modified time in UNIX timestamp format
                            let modified =
                                metadata.modified().ok().map(|mtime| {
                                    mtime
                                        .duration_since(std::time::UNIX_EPOCH)
                                        .map(|duration| duration.as_secs())
                                        .unwrap_or(0)
                                });

                            let file_object = FileObject::new(
                                file_name, file_size, modified, None,
                            );
                            file_objects.push(file_object);
                            *count += 1;
                        } else if recursive && metadata.is_dir() {
                            list_files_recursive(
                                &entry.path(),
                                max_keys,
                                recursive,
                                count,
                                file_objects,
                            );
                        }
                    }
                }
            }
        }
    }

    list_files_recursive(
        &path,
        max_keys,
        recursive,
        &mut count,
        &mut file_objects,
    );
    file_objects
}
