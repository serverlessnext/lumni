use std::error::Error;
use std::sync::Arc;
use std::{fmt, io};

use url::ParseError;

#[derive(Debug)]
pub enum InternalError {
    Io(io::Error),
    Parse(ParseError),
    String(String),
    ConfigError(String),
    NoBucketInUri(String),
    AccessDenied(String),
    InternalError(String),
    NotFound(String),
    Anyhow(anyhow::Error),
    Wrapped(Arc<dyn Error + Send + Sync + 'static>),
    #[cfg(target_arch = "wasm32")]
    Js(wasm_bindgen::JsValue),
    #[cfg(feature = "http_client")]
    HttpClientError(crate::http::client::HttpClientError),
}

impl fmt::Display for InternalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            InternalError::Io(e) => write!(f, "{}", e),
            InternalError::Parse(e) => write!(f, "{}", e),
            InternalError::String(s) => write!(f, "{}", s),
            InternalError::ConfigError(s) => write!(f, "Config error: {}", s),
            InternalError::Wrapped(e) => write!(f, "{}", e),
            InternalError::NoBucketInUri(s) => {
                write!(f, "No bucket specified in URI: {}", s)
            }
            InternalError::AccessDenied(s) => {
                write!(f, "Access denied: {}", s)
            }
            InternalError::InternalError(s) => {
                write!(f, "Internal error: {}", s)
            }
            InternalError::Anyhow(e) => write!(f, "Anyhow error: {}", e),
            InternalError::NotFound(s) => write!(f, "Not found: {}", s),
            #[cfg(target_arch = "wasm32")]
            InternalError::Js(e) => write!(
                f,
                "JsError: {}",
                e.as_string().unwrap_or_else(|| "Unknown error".to_string())
            ),
            #[cfg(feature = "http_client")]
            InternalError::HttpClientError(e) => write!(f, "{}", e),
        }
    }
}

impl Error for InternalError {}

impl From<Box<dyn Error + Send + Sync + 'static>> for InternalError {
    fn from(error: Box<dyn Error + Send + Sync + 'static>) -> Self {
        InternalError::Wrapped(Arc::from(error))
    }
}

impl From<Arc<dyn Error + Send + Sync + 'static>> for InternalError {
    fn from(error: Arc<dyn Error + Send + Sync + 'static>) -> Self {
        InternalError::Wrapped(error)
    }
}

impl From<io::Error> for InternalError {
    fn from(error: io::Error) -> Self {
        InternalError::Io(error)
    }
}

impl From<anyhow::Error> for InternalError {
    fn from(error: anyhow::Error) -> Self {
        InternalError::Anyhow(error)
    }
}

impl From<ParseError> for InternalError {
    fn from(error: ParseError) -> Self {
        InternalError::Parse(error)
    }
}

impl From<&str> for InternalError {
    fn from(error: &str) -> Self {
        InternalError::String(error.to_owned())
    }
}

impl From<std::string::String> for InternalError {
    fn from(error: std::string::String) -> Self {
        InternalError::String(error.to_owned())
    }
}

#[cfg(target_arch = "wasm32")]
impl From<wasm_bindgen::JsValue> for InternalError {
    fn from(error: wasm_bindgen::JsValue) -> Self {
        InternalError::Js(error)
    }
}
