use std::collections::HashMap;

use url::form_urlencoded;

use super::client::S3Client;
use crate::{LakestreamError, AWS_MAX_LIST_OBJECTS};

pub trait Headers {
    fn generate_list_buckets_headers(
        &self,
    ) -> Result<HashMap<String, String>, LakestreamError>;
    fn generate_list_objects_headers(
        &mut self,
        prefix: Option<&str>,
        max_keys: Option<u32>,
        continuation_token: Option<&str>,
    ) -> Result<HashMap<String, String>, LakestreamError>;
    fn generate_get_object_headers(
        &mut self,
        object_key: &str,
    ) -> Result<HashMap<String, String>, LakestreamError>;
    fn generate_head_object_headers(
        &mut self,
        object_key: &str,
    ) -> Result<HashMap<String, String>, LakestreamError>;
    fn create_list_objects_query_string(
        &self,
        prefix: Option<&str>,
        max_keys: Option<u32>,
        continuation_token: Option<&str>,
    ) -> String;
}

impl Headers for S3Client {
    fn generate_list_buckets_headers(
        &self,
    ) -> Result<HashMap<String, String>, LakestreamError> {
        let method = "GET";
        self.request_builder.generate_headers(
            self.config(),
            method,
            None,
            None,
            None,
        )
    }

    fn generate_list_objects_headers(
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

        self.query_string = query_string.clone();
        self.request_builder.generate_headers(
            self.config(),
            method,
            self.resource.as_deref(),
            query_string.as_deref(),
            None,
        )
    }

    fn generate_get_object_headers(
        &mut self,
        object_key: &str,
    ) -> Result<HashMap<String, String>, LakestreamError> {
        self.resource = Some(object_key.to_string());
        let method = "GET";
        self.request_builder.generate_headers(
            self.config(),
            method,
            self.resource.as_deref(),
            self.query_string.as_deref(),
            None,
        )
    }

    fn generate_head_object_headers(
        &mut self,
        object_key: &str,
    ) -> Result<HashMap<String, String>, LakestreamError> {
        self.resource = Some(object_key.to_string());
        let method = "HEAD";
        self.request_builder.generate_headers(
            self.config(),
            method,
            self.resource.as_deref(),
            None,
            None,
        )
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
