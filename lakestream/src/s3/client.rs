use std::collections::HashMap;

use hmac::{Hmac, Mac, NewMac};
use itertools::Itertools;
use percent_encoding::{utf8_percent_encode, CONTROLS};
use sha2::{Digest, Sha256};
use url::{form_urlencoded, Url};

use super::bucket::S3Credentials;
use crate::utils::time::UtcTimeNow;

const MAX_LIST_OBJECTS: u32 = 1000;

fn sign(key: &[u8], msg: &[u8]) -> Vec<u8> {
    let mut hmac = Hmac::<Sha256>::new_from_slice(key)
        .expect("HMAC can take key of any size");
    hmac.update(msg);
    let result = hmac.finalize();
    result.into_bytes().as_slice().to_vec()
}

pub struct S3Client {
    resource: String,
    region: String,
    credentials: S3Credentials,
    endpoint_url: String,
    utc_now: UtcTimeNow,
    query_string: Option<String>,
}

impl S3Client {
    pub fn new(
        endpoint_url: String,
        region: String,
        credentials: S3Credentials,
    ) -> S3Client {
        let resource = "".to_string();
        let utc_now = UtcTimeNow::new();

        S3Client {
            resource,
            region,
            credentials,
            endpoint_url,
            utc_now,
            query_string: None,
        }
    }

    pub fn url(&self) -> String {
        format!(
            "{}/{}?{}",
            &self.endpoint_url,
            &self.resource,
            self.query_string.as_ref().unwrap_or(&"".to_string())
        )
    }

    fn get_canonical_headers(
        &self,
        headers: &HashMap<String, String>,
    ) -> String {
        let mut canonical_headers = String::new();
        let mut headers_vec: Vec<(&String, &String)> = headers.iter().collect();
        headers_vec.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));

        for (header_name, header_value) in headers_vec {
            let header_name = header_name.trim().to_lowercase();
            if header_name.starts_with("x-amz-")
                && header_name != "x-amz-client-context"
                || header_name == "host"
                || header_name == "content-type"
                || header_name == "date"
            {
                canonical_headers +=
                    &format!("{}:{}\n", header_name, header_value.trim());
            }
        }

        canonical_headers
    }

    fn generate_signing_key(&self) -> Vec<u8> {
        let k_date = sign(
            format!("AWS4{}", self.credentials.secret_key()).as_bytes(),
            self.utc_now.date_stamp().as_bytes(),
        );
        let k_region = sign(&k_date, self.region.as_bytes());
        let k_service = sign(&k_region, b"s3");
        sign(&k_service, b"aws4_request")
    }

    fn initiate_headers(
        &self,
        headers: Option<HashMap<String, String>>,
        x_amz_date: &str,
        payload_hash: Option<&str>,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let mut headers = headers.unwrap_or_default();
        headers.insert("x-amz-date".to_string(), x_amz_date.to_string());
        headers.insert(
            "x-amz-content-sha256".to_string(),
            payload_hash.unwrap_or("UNSIGNED-PAYLOAD").to_string(),
        );
        Ok(headers)
    }

    fn get_canonical_uri(&self, url: &Url, resource: &str) -> String {
        let canonical_resource = form_urlencoded::byte_serialize(
            resource.trim_end_matches('/').as_bytes(),
        )
        .collect::<String>();
        let endpoint_path =
            url.path().trim_start_matches('/').trim_end_matches('/');

        if endpoint_path.is_empty() {
            canonical_resource
        } else {
            format!(
                "{}/{}",
                form_urlencoded::byte_serialize(endpoint_path.as_bytes())
                    .collect::<String>(),
                canonical_resource
            )
        }
    }

    fn get_canonical_query_string(
        &self,
    ) -> Result<String, Box<dyn std::error::Error>> {
        if self.query_string.as_ref().map_or(true, |s| s.is_empty()) {
            Ok(String::new())
        } else {
            let mut parts: Vec<(String, String)> =
                match self.query_string.as_ref() {
                    Some(query) => query
                        .split('&')
                        .filter_map(|p| {
                            let mut split = p.splitn(2, '=');
                            match (split.next(), split.next()) {
                                (Some(k), Some(v)) => {
                                    Some((k.to_string(), v.to_string()))
                                }
                                _ => None,
                            }
                        })
                        .collect(),
                    None => Vec::new(),
                };
            parts.sort();

            let encoded_parts: Vec<String> = parts
                .into_iter()
                .map(|(k, v)| {
                    format!("{}={}", k, utf8_percent_encode(&v, CONTROLS))
                })
                .collect();

            Ok(encoded_parts.join("&"))
        }
    }

    pub fn generate_list_buckets_headers(
        &mut self,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let method = "GET";

        self.generate_headers(method, None, None)
    }

    pub fn generate_list_objects_headers(
        &mut self,
        prefix: Option<&str>,
        max_keys: Option<u32>,
        continuation_token: Option<&str>,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let method = "GET";

        // Ensure max_keys does not exceed MAX_LIST_OBJECTS
        let max_keys = max_keys
            .map(|keys| std::cmp::min(keys, MAX_LIST_OBJECTS))
            .unwrap_or(MAX_LIST_OBJECTS);

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

        self.query_string = Some(query_parts.finish());

        self.generate_headers(method, None, None)
    }

    fn generate_headers(
        &mut self,
        method: &str,
        headers: Option<HashMap<String, String>>,
        payload_hash: Option<&str>,
    ) -> Result<HashMap<String, String>, Box<dyn std::error::Error>> {
        let date_stamp = self.utc_now.date_stamp();
        let x_amz_date = self.utc_now.x_amz_date();

        let credential_scope =
            format!("{}/{}/s3/aws4_request", date_stamp, self.region);
        let mut headers =
            self.initiate_headers(headers, &x_amz_date, payload_hash)?;

        let url = Url::parse(&self.endpoint_url)?;
        let host = url.host_str().ok_or("Missing host")?.to_owned();
        let host = match url.port() {
            Some(port) => host.replace(&format!(":{}", port), ""),
            None => host,
        };
        headers.insert("host".to_string(), host);

        let canonical_uri = self.get_canonical_uri(&url, &self.resource);

        let canonical_headers = self.get_canonical_headers(&headers);
        let signed_headers = headers
            .keys()
            .map(|key| key.to_lowercase())
            .sorted()
            .collect::<Vec<String>>()
            .join(";");

        let canonical_query_string = self.get_canonical_query_string()?;

        let canonical_request = format!(
            "{}\n/{}\n{}\n{}\n{}\n{}",
            method,
            canonical_uri,
            canonical_query_string,
            canonical_headers,
            signed_headers,
            payload_hash.unwrap_or("UNSIGNED-PAYLOAD")
        );

        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{:x}",
            x_amz_date,
            credential_scope,
            Sha256::digest(canonical_request.as_bytes())
        );

        let signing_key = self.generate_signing_key();
        let signature = sign(&signing_key, string_to_sign.as_bytes());

        let authorization_header = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            self.credentials.access_key(),
            credential_scope,
            signed_headers,
            hex::encode(signature)
        );

        headers.insert("Authorization".to_string(), authorization_header);
        Ok(headers)
    }
}
