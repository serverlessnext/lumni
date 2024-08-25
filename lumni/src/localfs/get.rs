use std::fs;
use std::io::Read;
use std::path::Path;

use crate::InternalError;

pub async fn get_object(
    path: &Path,
    key: &str,
    data: &mut Vec<u8>,
) -> Result<(), InternalError> {
    let object_path = path.join(key);

    if object_path.is_file() {
        let mut file = fs::File::open(&object_path).map_err(|err| {
            InternalError::InternalError(format!(
                "Failed to open file {}: {}",
                object_path.display(),
                err
            ))
        })?;

        file.read_to_end(data).map_err(|err| {
            InternalError::InternalError(format!(
                "Failed to read file {}: {}",
                object_path.display(),
                err
            ))
        })?;

        Ok(())
    } else {
        Err(InternalError::NotFound(format!(
            "Object not found for key: {}",
            key
        )))
    }
}

pub async fn head_object(path: &Path, key: &str) -> Result<(), InternalError> {
    let object_path = path.join(key);

    if object_path.is_file() {
        Ok(())
    } else {
        Err(InternalError::NotFound(format!(
            "Object not found for key: {}",
            key
        )))
    }
}
