pub use super::client_config::S3ClientConfig;
use super::request_builder::RequestBuilder;

pub struct S3Client {
    pub resource: Option<String>,
    pub config: S3ClientConfig,
    pub query_string: Option<String>,
    pub request_builder: RequestBuilder,
}

impl S3Client {
    pub fn new(config: S3ClientConfig) -> S3Client {
        log::info!(
            "S3Client created with endpoint_url: {}",
            config.bucket_url()
        );

        // Initialize RequestBuilder
        let request_builder = RequestBuilder::new(&config.bucket_url());

        S3Client {
            resource: None,
            config,
            query_string: None,
            request_builder,
        }
    }

    pub fn config(&self) -> &S3ClientConfig {
        &self.config
    }

    pub fn region(&self) -> &str {
        self.config.region()
    }

    pub fn url(&self) -> String {
        let mut url = format!(
            "{}/{}",
            self.config.bucket_url(),
            self.resource.as_ref().unwrap_or(&"".to_string())
        );
    
        if let Some(query) = self.query_string.as_ref() {
            url.push('?');
            url.push_str(query);
        }
        url
    }
    

}

impl Clone for S3Client {
    fn clone(&self) -> Self {
        S3Client::new(self.config.clone())
    }
}
