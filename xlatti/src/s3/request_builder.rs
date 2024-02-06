use std::collections::HashMap;

use hmac::{Hmac, Mac, NewMac};
use percent_encoding::{utf8_percent_encode, CONTROLS};
use sha2::{Digest, Sha256};
use url::{form_urlencoded, Url};

use super::client::S3ClientConfig;
use crate::utils::time::UtcTimeNow;
use crate::LakestreamError;

pub struct RequestBuilder {
    url: String,
}

impl RequestBuilder {
    pub fn new(url: &str) -> Self {
        let url = url.to_string();
        Self { url }
    }

    pub fn generate_headers(
        &self,
        config: &S3ClientConfig,
        method: &str,
        resource: Option<&str>,
        query_string: Option<&str>,
        payload_hash: Option<&str>,
    ) -> Result<HashMap<String, String>, LakestreamError> {
        let utc_now = UtcTimeNow::new();
        let date_stamp = utc_now.date_stamp();
        let x_amz_date = utc_now.x_amz_date();

        let credential_scope =
            format!("{}/{}/s3/aws4_request", date_stamp, config.region());
        let mut headers = self.initiate_headers(&x_amz_date, payload_hash);

        let url = Url::parse(&self.url)?;
        let host = url.host_str().ok_or("Missing host")?.to_owned();
        let host = match url.port() {
            Some(port) => format!("{}:{}", host, port),
            None => host,
        };

        headers.insert("host".to_string(), host);

        let canonical_uri = self.get_canonical_uri(&url, resource);

        let canonical_headers = self.get_canonical_headers(&headers);

        let mut signed_headers: Vec<String> =
            headers.keys().map(|key| key.to_lowercase()).collect();

        signed_headers.sort();

        let signed_headers_str = signed_headers.join(";");

        let canonical_query_string =
            self.get_canonical_query_string(query_string)?;

        let canonical_request = format!(
            "{}\n/{}\n{}\n{}\n{}\n{}",
            method,
            canonical_uri,
            canonical_query_string,
            canonical_headers,
            signed_headers_str,
            payload_hash.unwrap_or("UNSIGNED-PAYLOAD")
        );

        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{:x}",
            x_amz_date,
            credential_scope,
            Sha256::digest(canonical_request.as_bytes())
        );
        let signing_key = self.generate_signing_key(
            &date_stamp,
            config.credentials().secret_key(),
            config.region(),
        );
        let signature = sign(&signing_key, string_to_sign.as_bytes());

        let authorization_header = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            config.credentials().access_key(),
            credential_scope,
            signed_headers_str,
            hex::encode(signature)
        );

        headers.insert("Authorization".to_string(), authorization_header);
        Ok(headers)
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

    fn generate_signing_key(
        &self,
        date_stamp: &str,
        secret_key: &str,
        region: &str,
    ) -> Vec<u8> {
        let k_date = sign(
            format!("AWS4{}", secret_key).as_bytes(),
            date_stamp.as_bytes(),
        );
        let k_region = sign(&k_date, region.as_bytes());
        let k_service = sign(&k_region, b"s3");
        sign(&k_service, b"aws4_request")
    }

    fn initiate_headers(
        &self,
        x_amz_date: &str,
        payload_hash: Option<&str>,
    ) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("x-amz-date".to_string(), x_amz_date.to_string());
        headers.insert(
            "x-amz-content-sha256".to_string(),
            payload_hash.unwrap_or("UNSIGNED-PAYLOAD").to_string(),
        );
        headers
    }

    fn get_canonical_uri(&self, url: &Url, resource: Option<&str>) -> String {
        let canonical_resource = form_urlencoded::byte_serialize(
            resource
                .unwrap_or_default()
                .trim_start_matches('/')
                .trim_end_matches('/')
                .as_bytes(),
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
        query_string: Option<&str>,
    ) -> Result<String, LakestreamError> {
        if query_string.as_ref().map_or(true, |s| s.is_empty()) {
            Ok(String::new())
        } else {
            let mut parts: Vec<(String, String)> = match query_string.as_ref() {
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
}

fn sign(key: &[u8], msg: &[u8]) -> Vec<u8> {
    let mut hmac = Hmac::<Sha256>::new_from_slice(key)
        .expect("HMAC can take key of any size");
    hmac.update(msg);
    let result = hmac.finalize();
    result.into_bytes().as_slice().to_vec()
}
