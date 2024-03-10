use log::info;

use super::bucket::S3Bucket;
use super::client_headers::Headers;
use super::list::create_s3_client;
use super::request_handler::http_with_redirect_handling;
use crate::{LakestreamError, ObjectStoreTrait};

pub async fn get_object(
    s3_bucket: &S3Bucket,
    object_key: &str,
    data: &mut Vec<u8>,
) -> Result<(), LakestreamError> {
    let s3_client =
        create_s3_client(s3_bucket.config(), Some(s3_bucket.name()));

    info!("Getting object: {}", object_key);
    let (body_bytes, _updated_s3_client, _status_code, _response_headers) =
        http_with_redirect_handling(
            &s3_client,
            |s3_client| s3_client.generate_get_object_headers(object_key),
            "GET",
        )
        .await?;
    info!(
        "Got object: {} of size {} bytes",
        object_key,
        body_bytes.len()
    );
    // Write response body directly into the provided Vec<u8>
    data.clear();
    data.extend_from_slice(&body_bytes);

    Ok(())
}
