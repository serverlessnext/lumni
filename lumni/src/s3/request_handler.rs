use std::collections::HashMap;

use bytes::Bytes;

use crate::http::requests::http_request_with_headers;
use crate::s3::client::S3Client;
use crate::s3::client_config::S3ClientConfig;
use crate::InternalError;

async fn handle_redirect(s3_client: &S3Client, new_region: &str) -> S3Client {
    let config = s3_client.config();
    let bucket_name = config.bucket_name();
    let credentials = config.credentials().clone();
    let endpoint_url = config.endpoint_url();

    let s3_client_config =
        S3ClientConfig::new(credentials, bucket_name, endpoint_url, new_region);
    S3Client::new(s3_client_config)
}

pub async fn http_with_redirect_handling<F>(
    s3_client: &S3Client,
    generate_headers: F,
    method: &str,
) -> Result<
    (Bytes, Option<S3Client>, u16, HashMap<String, String>),
    InternalError,
>
where
    F: Fn(&mut S3Client) -> Result<HashMap<String, String>, InternalError>,
{
    let mut current_s3_client = s3_client.clone();
    loop {
        let headers = generate_headers(&mut current_s3_client)?;
        let result = http_request_with_headers(
            &current_s3_client.url(),
            &headers,
            method,
        )
        .await;

        match result {
            Ok((body_bytes, status_code, response_headers)) => {
                if status_code == 301 {
                    if let Some(new_region) =
                        response_headers.get("x-amz-bucket-region")
                    {
                        current_s3_client =
                            handle_redirect(&current_s3_client, new_region)
                                .await;
                    } else {
                        let error = "Error: Redirect without \
                                     x-amz-bucket-region header";
                        return Err(InternalError::from(error));
                    }
                } else {
                    if status_code == 403 {
                        let url = current_s3_client.url();
                        return Err(InternalError::AccessDenied(
                            url.to_string(),
                        ));
                    }

                    // TODO: Handle non-200 status codes
                    // TODO: return response_headers to accomodate HEAD requests
                    return Ok((
                        body_bytes,
                        if current_s3_client.region() != s3_client.region() {
                            Some(current_s3_client)
                        } else {
                            None
                        },
                        status_code,
                        response_headers,
                    ));
                }
            }
            Err(e) => return Err(InternalError::from(e)),
        }
    }
}
