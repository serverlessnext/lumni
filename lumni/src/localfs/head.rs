use std::path::Path;

use crate::InternalError;

pub async fn head_object(path: &Path, key: &str) -> Result<(), InternalError> {
    let object_path = path.join(key);
    // TODO:
    // currently just check if file exists
    // should return metadata
    if object_path.is_file() {
        Ok(())
    } else {
        Err(InternalError::NotFound(format!(
            "Object not found for key: {}",
            key
        )))
    }
}
