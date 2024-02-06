// localfs/get.rs

use std::fs;
use std::io::Read;
use std::path::Path;

use crate::LakestreamError;

pub async fn get_object(
    path: &Path,
    key: &str,
    data: &mut Vec<u8>,
) -> Result<(), LakestreamError> {
    let object_path = path.join(key);

    if object_path.is_file() {
        let mut file = fs::File::open(&object_path).map_err(|err| {
            LakestreamError::InternalError(format!(
                "Failed to open file {}: {}",
                object_path.display(),
                err
            ))
        })?;

        file.read_to_end(data).map_err(|err| {
            LakestreamError::InternalError(format!(
                "Failed to read file {}: {}",
                object_path.display(),
                err
            ))
        })?;

        Ok(())
    } else {
        Err(LakestreamError::NotFound(format!(
            "Object not found for key: {}",
            key
        )))
    }
}
