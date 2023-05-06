// localfs/get.rs

use std::fs;
use std::io::Read;
use std::path::Path;

use crate::LakestreamError;

pub fn get_object(path: &Path, key: &str) -> Result<String, LakestreamError> {
    let object_path = path.join(key);

    if object_path.is_file() {
        let mut file = fs::File::open(&object_path).map_err(|err| {
            LakestreamError::InternalError(format!(
                "Failed to open file {}: {}",
                object_path.display(),
                err
            ))
        })?;

        let mut content = String::new();
        file.read_to_string(&mut content).map_err(|err| {
            LakestreamError::InternalError(format!(
                "Failed to read file {}: {}",
                object_path.display(),
                err
            ))
        })?;

        Ok(content)
    } else {
        Err(LakestreamError::NotFound(format!(
            "Object not found for key: {}",
            key
        )))
    }
}
