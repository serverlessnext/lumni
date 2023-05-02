use std::collections::HashMap;

use url::form_urlencoded;

use super::bucket::S3Credentials;
use super::request_builder::RequestBuilder;
use crate::{LakestreamError, AWS_MAX_LIST_OBJECTS};

#[derive(Clone)]
pub struct S3ClientConfig {
    credentials: S3Credentials,
    endpoint_url: String,
    region: String,
}

impl S3ClientConfig {
    pub fn new(
        credentials: S3Credentials,
        bucket_url: &str,
        region: &str,
    ) -> S3ClientConfig {
        S3ClientConfig {
            credentials,
            endpoint_url: bucket_url.to_string(),
            region: region.to_string(),
        }
    }

    pub fn credentials(&self) -> &S3Credentials {
        &self.credentials
    }

    pub fn region(&self) -> &str {
        &self.region
    }
}

pub struct S3Client {
    resource: Option<String>,
    config: S3ClientConfig,
    query_string: Option<String>,
    request_builder: RequestBuilder,
}

impl S3Client {
    pub fn new(config: S3ClientConfig) -> S3Client {
        log::info!(
            "S3Client created with endpoint_url: {}",
            config.endpoint_url
        );

        // Initialize RequestBuilder
        let request_builder = RequestBuilder::new(&config.endpoint_url);

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

    pub fn credentials(&self) -> &S3Credentials {
        &self.config.credentials
    }

    pub fn region(&self) -> &str {
        &self.config.region
    }

    pub fn url(&self) -> String {
        format!(
            "{}/{}?{}",
            &self.config.endpoint_url,
            self.resource.as_ref().unwrap_or(&"".to_string()),
            self.query_string.as_ref().unwrap_or(&"".to_string())
        )
    }

    fn generate_headers(
        &mut self,
        method: &str,
        query_string: Option<String>,
    ) -> Result<HashMap<String, String>, LakestreamError> {
        // update self.query_string - its still used for the pub url() method
        // when pub url() can generate its own query_string, this can be removed
        // so we can remove &mut self from this method
        self.query_string = query_string;
        self.request_builder.generate_headers(
            &self.config,
            method,
            self.resource.as_deref(),
            self.query_string.as_deref(),
            None,
        )
    }

    pub fn generate_list_buckets_headers(
        &mut self,
    ) -> Result<HashMap<String, String>, LakestreamError> {
        let method = "GET";
        self.generate_headers(method, None)
    }

    pub fn generate_list_objects_headers(
        &mut self,
        prefix: Option<&str>,
        max_keys: Option<u32>,
        continuation_token: Option<&str>,
    ) -> Result<HashMap<String, String>, LakestreamError> {
        let method = "GET";
        let query_string = Some(self.create_list_objects_query_string(
            prefix,
            max_keys,
            continuation_token,
        ));
        self.generate_headers(method, query_string)
    }

    fn create_list_objects_query_string(
        &self,
        prefix: Option<&str>,
        max_keys: Option<u32>,
        continuation_token: Option<&str>,
    ) -> String {
        // Ensure max_keys does not exceed AWS_MAX_LIST_OBJECTS
        let max_keys = max_keys
            .map(|keys| std::cmp::min(keys, AWS_MAX_LIST_OBJECTS))
            .unwrap_or(AWS_MAX_LIST_OBJECTS);

        let mut query_parts = form_urlencoded::Serializer::new(String::new());
        query_parts.append_pair("list-type", "2");
        query_parts.append_pair("max-keys", &max_keys.to_string());
        query_parts.append_pair("delimiter", "/");
        query_parts.append_pair("encoding-type", "url");

        if let Some(p) = prefix {
            query_parts.append_pair("prefix", p);
        }
        if let Some(token) = continuation_token {
            query_parts.append_pair("continuation-token", token);
        }

        query_parts.finish()
    }
}

impl Clone for S3Client {
    fn clone(&self) -> Self {
        S3Client::new(self.config.clone())
    }
}
