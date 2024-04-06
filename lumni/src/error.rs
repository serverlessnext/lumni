use std::error::Error;
use std::{fmt, io};

use url::ParseError;

#[derive(Debug)]
pub enum LakestreamError {
    Io(io::Error),
    Parse(ParseError),
    String(String),
    ConfigError(String),
    NoBucketInUri(String),
    AccessDenied(String),
    InternalError(String),
    NotFound(String),
    Anyhow(anyhow::Error),
    Wrapped(Box<dyn Error + 'static>),
    #[cfg(target_arch = "wasm32")]
    Js(wasm_bindgen::JsValue),
    #[cfg(feature = "http_client")]
    HttpClientError(crate::http::HttpClientError),
}

impl fmt::Display for LakestreamError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LakestreamError::Io(e) => write!(f, "{}", e),
            LakestreamError::Parse(e) => write!(f, "{}", e),
            LakestreamError::String(s) => write!(f, "{}", s),
            LakestreamError::ConfigError(s) => write!(f, "Config error: {}", s),
            LakestreamError::Wrapped(e) => write!(f, "{}", e),
            LakestreamError::NoBucketInUri(s) => {
                write!(f, "No bucket specified in URI: {}", s)
            }
            LakestreamError::AccessDenied(s) => {
                write!(f, "Access denied: {}", s)
            }
            LakestreamError::InternalError(s) => {
                write!(f, "Internal error: {}", s)
            }
            LakestreamError::Anyhow(e) => write!(f, "Anyhow error: {}", e),
            LakestreamError::NotFound(s) => write!(f, "Not found: {}", s),
            #[cfg(target_arch = "wasm32")]
            LakestreamError::Js(e) => write!(
                f,
                "JsError: {}",
                e.as_string().unwrap_or_else(|| "Unknown error".to_string())
            ),
            #[cfg(feature = "http_client")]
            LakestreamError::HttpClientError(e) => write!(f, "{}", e),
        }
    }
}

impl Error for LakestreamError {}

impl From<Box<dyn Error>> for LakestreamError {
    fn from(error: Box<dyn Error>) -> Self {
        LakestreamError::Wrapped(error)
    }
}

impl From<io::Error> for LakestreamError {
    fn from(error: io::Error) -> Self {
        LakestreamError::Io(error)
    }
}

impl From<anyhow::Error> for LakestreamError {
    fn from(error: anyhow::Error) -> Self {
        LakestreamError::Anyhow(error)
    }
}

impl From<ParseError> for LakestreamError {
    fn from(error: ParseError) -> Self {
        LakestreamError::Parse(error)
    }
}

impl From<&str> for LakestreamError {
    fn from(error: &str) -> Self {
        LakestreamError::String(error.to_owned())
    }
}

impl From<std::string::String> for LakestreamError {
    fn from(error: std::string::String) -> Self {
        LakestreamError::String(error.to_owned())
    }
}

#[cfg(target_arch = "wasm32")]
impl From<wasm_bindgen::JsValue> for LakestreamError {
    fn from(error: wasm_bindgen::JsValue) -> Self {
        LakestreamError::Js(error)
    }
}
