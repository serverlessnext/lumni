use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt;
use std::str::FromStr;
use std::string::FromUtf8Error;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Error as AnyhowError, Result};
use bytes::{Bytes, BytesMut};
use futures::future::pending;
use http_body_util::combinators::BoxBody;
use http_body_util::{BodyExt, Empty, Full};
use hyper::header::{HeaderName, HeaderValue};
use hyper::{HeaderMap, Request, Uri};
use hyper_tls::HttpsConnector;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::client::legacy::Client;
use hyper_util::rt::TokioExecutor;
use percent_encoding::{percent_encode, utf8_percent_encode, NON_ALPHANUMERIC};
use serde::de::DeserializeOwned;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
pub struct HttpClientResponse {
    body: Option<Bytes>,
    status_code: u16,
    headers: HeaderMap,
}

impl HttpClientResponse {
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
    RequestCancelled,
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
            HttpClientError::RequestCancelled => write!(f, "Request cancelled"),
        }
    }
}
pub trait HttpClientErrorHandler {
    fn handle_error(
        &self,
        response: HttpClientResponse,
        canonical_reason: String,
    ) -> HttpClientError;
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

impl HttpClientResponse {
    pub fn json<T: DeserializeOwned>(&self) -> Result<T> {
        if let Some(body) = &self.body {
            serde_json::from_slice(body).map_err(|e| anyhow!(e))
        } else {
            Err(anyhow!("No body"))
        }
    }
}

pub type HttpClientResult = Result<HttpClientResponse, HttpClientError>;

#[derive(Clone)]
pub struct HttpClient {
    client: Client<
        HttpsConnector<HttpConnector>,
        BoxBody<bytes::Bytes, Infallible>,
    >,
    timeout: Duration,
    error_handler: Option<Arc<dyn HttpClientErrorHandler + Send + Sync>>,
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
            error_handler: None,
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_error_handler(
        mut self,
        error_handler: Arc<dyn HttpClientErrorHandler + Send + Sync>,
    ) -> Self {
        self.error_handler = Some(error_handler);
        self
    }

    async fn request(
        &self,
        method: &str,
        url: &str,
        headers: Option<&HashMap<String, String>>,
        body: Option<&Bytes>,
        tx: Option<mpsc::Sender<Bytes>>,
        mut cancel_rx: Option<oneshot::Receiver<()>>,
    ) -> HttpClientResult {
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
            let canonical_reason = response
                .status()
                .canonical_reason()
                .unwrap_or("")
                .to_string();
            if let Some(error_handler) = &self.error_handler {
                // Custom error handling
                let http_client_response = HttpClientResponse {
                    body: None,
                    status_code: response.status().as_u16(),
                    headers: response.headers().clone(),
                };
                return Err(error_handler
                    .handle_error(http_client_response, canonical_reason));
            }
            return Err(HttpClientError::HttpError(
                response.status().as_u16(),
                canonical_reason,
            ));
        }

        let status_code = response.status().as_u16();
        let headers = response.headers().clone();

        let body;

        if let Some(tx) = &tx {
            body = None;

            eprintln!("Headers={:?}", response.headers());
            loop {
                let frame_future = response.frame();
                tokio::select! {
                    next = frame_future => {
                        match next {
                            Some(Ok(frame)) => {
                                if let Ok(chunk) = frame.into_data() {
                                    if let Err(e) = tx.send(chunk).await {
                                        return Err(HttpClientError::Other(e.into()));
                                    }
                                }
                            },
                            Some(Err(e)) => return Err(HttpClientError::Other(e.into())),
                            None => break, // End of the stream
                        }
                    },
                    // Check if the request has been cancelled
                    _ = async {
                        if let Some(rx) = &mut cancel_rx {
                            rx.await.ok();
                        } else {
                            pending::<()>().await;
                        }
                    } => {
                        drop(response); // Optionally drop the response to close the connection
                        return Err(HttpClientError::RequestCancelled);
                    },
                }
            }
        } else {
            let mut body_bytes = BytesMut::new();
            while let Some(next) = response.frame().await {
                // get headers for debugging
                let frame = next.map_err(|e| anyhow!(e))?;
                if let Some(chunk) = frame.data_ref() {
                    body_bytes.extend_from_slice(chunk);
                }
            }
            body = Some(body_bytes.into());
        }

        Ok(HttpClientResponse {
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
        cancel_rx: Option<oneshot::Receiver<()>>,
    ) -> HttpClientResult {
        self.request("GET", url, headers, None, tx, cancel_rx).await
    }

    pub async fn post(
        &self,
        url: &str,
        headers: Option<&HashMap<String, String>>,
        _params: Option<&HashMap<String, String>>,
        body: Option<&Bytes>,
        tx: Option<mpsc::Sender<Bytes>>,
        cancel_rx: Option<oneshot::Receiver<()>>,
    ) -> HttpClientResult {
        self.request("POST", url, headers, body, tx, cancel_rx)
            .await
    }
}

// additional non associated helper functions
impl HttpClient {
    pub fn percent_encode_with_exclusion(
        input: &str,
        exclude: Option<&[u8]>,
    ) -> String {
        let mut result = String::new();
        let set = NON_ALPHANUMERIC;

        if let Some(exclusions) = exclude {
            // percent-encode each byte while skipping excluded characters
            for byte in input.bytes() {
                if exclusions.contains(&byte) {
                    result.push(byte as char);
                } else {
                    result.push_str(
                        &percent_encode(&[byte][..], &set).to_string(),
                    );
                }
            }
        } else {
            // use the standard percent encoding for the entire input
            result.push_str(&utf8_percent_encode(input, &set).to_string());
        }
        result
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
