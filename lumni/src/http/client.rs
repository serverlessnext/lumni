use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt;
use std::str::FromStr;
use std::string::FromUtf8Error;
use std::time::Duration;

use anyhow::{anyhow, Error as AnyhowError, Result};
use bytes::{Bytes, BytesMut};
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full};
use hyper::header::{HeaderName, HeaderValue};
use hyper::{HeaderMap, Request, Uri};
use hyper_tls::HttpsConnector;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use serde::de::DeserializeOwned;
use tokio::sync::mpsc;

pub struct HttpResponse {
    body: Option<Bytes>,
    status_code: u16,
    headers: HeaderMap,
}

impl HttpResponse {
    pub fn body(&self) -> Option<&Bytes> {
        self.body.as_ref()
    }

    pub fn status_code(&self) -> u16 {
        self.status_code
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }
}

#[derive(Debug)]
pub enum HttpClientError {
    ConnectionError(anyhow::Error),
    TimeoutError,
    HttpError(u16, String), // Status code, status text
    Utf8Error(String),
    Other(AnyhowError),
}

impl fmt::Display for HttpClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HttpClientError::ConnectionError(e) => {
                write!(f, "Connection error: {}", e)
            }
            HttpClientError::TimeoutError => write!(f, "Timeout error"),
            HttpClientError::HttpError(code, message) => {
                write!(f, "HTTP error {}: {}", code, message)
            }
            HttpClientError::Utf8Error(e) => write!(f, "UTF-8 error: {}", e),
            HttpClientError::Other(e) => write!(f, "Other error: {}", e),
        }
    }
}

impl From<hyper::http::Error> for HttpClientError {
    fn from(err: hyper::http::Error) -> Self {
        HttpClientError::Other(AnyhowError::new(err))
    }
}

impl From<AnyhowError> for HttpClientError {
    fn from(err: AnyhowError) -> Self {
        HttpClientError::Other(err)
    }
}

impl From<FromUtf8Error> for HttpClientError {
    fn from(err: FromUtf8Error) -> Self {
        HttpClientError::Utf8Error(err.to_string())
    }
}

impl HttpResponse {
    pub fn json<T: DeserializeOwned>(&self) -> Result<T> {
        if let Some(body) = &self.body {
            serde_json::from_slice(body).map_err(|e| anyhow!(e))
        } else {
            Err(anyhow!("No body"))
        }
    }
}

pub type HttpResult = Result<HttpResponse, HttpClientError>;

#[derive(Clone)]
pub struct HttpClient {
    client: Client<
        HttpsConnector<HttpConnector>,
        BoxBody<bytes::Bytes, Infallible>,
    >,
    timeout: Duration,
}

impl HttpClient {
    pub fn new() -> Self {
        let https = HttpsConnector::new();

        let client: Client<
            HttpsConnector<HttpConnector>,
            BoxBody<Bytes, Infallible>,
        > = Client::builder(TokioExecutor::new())
            .build::<_, BoxBody<Bytes, Infallible>>(https);

        HttpClient {
            client,
            timeout: Duration::from_secs(30),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    async fn request(
        &self,
        method: &str,
        url: &str,
        headers: Option<&HashMap<String, String>>,
        body: Option<&Bytes>,
        tx: Option<mpsc::Sender<Bytes>>,
    ) -> HttpResult {
        let uri = Uri::from_str(url)
            .map_err(|e| HttpClientError::Other(AnyhowError::new(e)))?;

        let mut req_builder = Request::builder().method(method).uri(uri);

        if let Some(headers_map) = headers {
            for (key, value) in headers_map.iter() {
                let header_name =
                    HeaderName::from_str(key).expect("Invalid header name");
                let header_value =
                    HeaderValue::from_str(value).expect("Invalid header value");
                req_builder = req_builder.header(header_name, header_value);
            }
        }

        let request_body = create_request_body(body);

        let request = req_builder
            .body(request_body)
            .expect("Failed to build the request");

        // Send the request and await the response, handling timeout as needed
        let mut response = self.client.request(request).await.map_err(|e| {
            HttpClientError::ConnectionError(AnyhowError::new(e))
        })?;
        if !response.status().is_success() {
            return Err(HttpClientError::HttpError(
                response.status().as_u16(),
                format!(
                    "{}",
                    response.status().canonical_reason().unwrap_or("")
                ),
            ));
        }

        let status_code = response.status().as_u16();
        let headers = response.headers().clone();

        let body;

        if let Some(tx) = &tx {
            body = None;
            while let Some(next) = response.frame().await {
                let frame = next.map_err(|e| anyhow!(e))?;
                if let Ok(chunk) = frame.into_data() {
                    tx.send(chunk).await.map_err(|_| {
                        anyhow!("Failed to send data via channel")
                    })?;
                }
            }
        } else {
            let mut body_bytes = BytesMut::new();
            while let Some(next) = response.frame().await {
                let frame = next.map_err(|e| anyhow!(e))?;
                if let Some(chunk) = frame.data_ref() {
                    body_bytes.extend_from_slice(chunk);
                }
            }
            body = Some(body_bytes.into());
        }

        Ok(HttpResponse {
            body,
            status_code,
            headers,
        })
    }

    pub async fn get(
        &self,
        url: &str,
        headers: Option<&HashMap<String, String>>,
        _params: Option<&HashMap<String, String>>,
        tx: Option<mpsc::Sender<Bytes>>,
    ) -> HttpResult {
        self.request("GET", url, headers, None, tx).await
    }

    pub async fn post(
        &self,
        url: &str,
        headers: Option<&HashMap<String, String>>,
        _params: Option<&HashMap<String, String>>,
        body: Option<&Bytes>,
        tx: Option<mpsc::Sender<Bytes>>,
    ) -> HttpResult {
        self.request("POST", url, headers, body, tx).await
    }
}

fn create_request_body(
    body_content: Option<&Bytes>,
) -> BoxBody<Bytes, Infallible> {
    match body_content {
        Some(content) => {
            let full_body: Full<Bytes> = Full::new(content.clone());
            BoxBody::new(full_body)
        }
        None => {
            let empty_body: Empty<Bytes> = Empty::new();
            BoxBody::new(empty_body)
        }
    }
}
