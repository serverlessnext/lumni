use crate::base::config::Config;
use crate::http::requests::http_get_request_with_headers;
use crate::s3::bucket::configure_bucket_url;
use crate::s3::client::{S3Client, S3ClientConfig};
use crate::LakestreamError;

async fn handle_redirect(
    s3_bucket_config: &Config,
    s3_bucket_name: &str,
    s3_client: &S3Client,
    new_region: &str,
) -> S3Client {
    let endpoint_url = s3_bucket_config
        .settings
        .get("S3_ENDPOINT_URL")
        .map(String::as_str);
    let bucket_url =
        configure_bucket_url(new_region, endpoint_url, Some(s3_bucket_name));
    let credentials = s3_client.credentials().clone();
    let s3_client_config =
        S3ClientConfig::new(credentials, &bucket_url, new_region);
    S3Client::new(s3_client_config)
}

pub async fn http_get_with_redirect_handling(
    s3_bucket_config: &Config,
    s3_bucket_name: &str,
    s3_client: &S3Client,
    prefix: Option<&str>,
    max_keys: Option<u32>,
    continuation_token: Option<&str>,
) -> Result<(String, Option<S3Client>), LakestreamError> {
    let mut current_s3_client = s3_client.clone();

    loop {
        let headers = &current_s3_client
            .generate_list_objects_headers(prefix, max_keys, continuation_token)
            .unwrap();

        let result =
            http_get_request_with_headers(&current_s3_client.url(), headers)
                .await;

        match result {
            Ok((response_body, status, response_headers)) => {
                if status == 301 {
                    if let Some(new_region) =
                        response_headers.get("x-amz-bucket-region")
                    {
                        current_s3_client = handle_redirect(
                            s3_bucket_config,
                            s3_bucket_name,
                            &current_s3_client,
                            new_region,
                        )
                        .await;
                    } else {
                        let error = "Error: Redirect without \
                                     x-amz-bucket-region header";
                        return Err(LakestreamError::from(error));
                    }
                } else {
                    // TODO: Handle non-200 status codes
                    return Ok((
                        response_body,
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
