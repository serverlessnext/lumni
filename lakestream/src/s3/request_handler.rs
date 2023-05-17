use std::collections::HashMap;

use bytes::Bytes;

use crate::http::requests::http_get_request_with_headers;
use crate::s3::client::{S3Client, S3ClientConfig};
use crate::LakestreamError;

async fn handle_redirect(s3_client: &S3Client, new_region: &str) -> S3Client {
    let config = s3_client.config();
    let bucket_name = config.bucket_name();
    let credentials = config.credentials().clone();
    let endpoint_url = config.endpoint_url();

    let s3_client_config =
        S3ClientConfig::new(credentials, bucket_name, endpoint_url, new_region);
    S3Client::new(s3_client_config)
}

pub async fn http_get_with_redirect_handling<F>(
    s3_client: &S3Client,
    generate_headers: F,
) -> Result<(Bytes, Option<S3Client>), LakestreamError>
where
    F: Fn(&mut S3Client) -> Result<HashMap<String, String>, LakestreamError>,
{
    let mut current_s3_client = s3_client.clone();

    loop {
        let headers = generate_headers(&mut current_s3_client)?;
        let result =
            http_get_request_with_headers(&current_s3_client.url(), &headers)
                .await;

        match result {
            Ok((body_bytes, status, response_headers)) => {
                if status == 301 {
                    if let Some(new_region) =
                        response_headers.get("x-amz-bucket-region")
                    {
                        current_s3_client =
                            handle_redirect(&current_s3_client, new_region)
                                .await;
                    } else {
                        let error = "Error: Redirect without \
                                     x-amz-bucket-region header";
                        return Err(LakestreamError::from(error));
                    }
                } else {
                    // TODO: Handle non-200 status codes
                    return Ok((
                        body_bytes,
                        if current_s3_client.region() != s3_client.region() {
                            Some(current_s3_client)
                        } else {
                            None
                        },
                    ));
                }
            }
            Err(e) => return Err(LakestreamError::from(e)),
        }
    }
}
