use std::collections::HashMap;

use log::info;

use super::bucket::S3Bucket;
use super::client_headers::Headers;
use super::list::create_s3_client;
use super::request_handler::http_with_redirect_handling;
use crate::{LakestreamError, ObjectStoreTrait};

pub async fn head_object(
    s3_bucket: &S3Bucket,
    object_key: &str,
) -> Result<(HashMap<String, String>), LakestreamError> {
    let s3_client =
        create_s3_client(s3_bucket.config(), Some(s3_bucket.name()));

    info!("Head object: {}", object_key);

    // TODO: head does not return a body, we need to update http_with_redirect_handling
    // to return response headers as a separate object
    let (_body_bytes, _updated_s3_client, response_headers) =
        http_with_redirect_handling(&s3_client, |s3_client| {
            s3_client.generate_head_object_headers(object_key)
        },
        "HEAD").await?;
    Ok(response_headers)
}