use super::bucket::{configure_bucket_url, S3Credentials};

#[derive(Clone)]
pub struct S3ClientConfig {
    credentials: S3Credentials,
    bucket_name: Option<String>,
    endpoint_url: Option<String>,
    region: String,
}

impl S3ClientConfig {
    pub fn new(
        credentials: S3Credentials,
        bucket_name: Option<&str>,
        endpoint_url: Option<&str>,
        region: &str,
    ) -> S3ClientConfig {
        S3ClientConfig {
            credentials,
            bucket_name: bucket_name.map(str::to_string),
            endpoint_url: endpoint_url.map(str::to_string),
            region: region.to_string(),
        }
    }

    pub fn credentials(&self) -> &S3Credentials {
        &self.credentials
    }

    pub fn bucket_name(&self) -> Option<&str> {
        self.bucket_name.as_deref()
    }

    pub fn endpoint_url(&self) -> Option<&str> {
        self.endpoint_url.as_deref()
    }

    pub fn region(&self) -> &str {
        &self.region
    }

    pub fn bucket_url(&self) -> String {
        configure_bucket_url(
            self.region(),
            self.endpoint_url.as_deref(),
            self.bucket_name.as_deref(),
        )
    }
}
