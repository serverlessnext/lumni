use std::error::Error;
use std::{fmt, io};

use url::ParseError;

#[derive(Debug)]
pub enum LakestreamError {
    Io(io::Error),
    Parse(ParseError),
    String(String),
    #[cfg(target_arch = "wasm32")]
    Js(wasm_bindgen::JsValue),
    Wrapped(Box<dyn Error + 'static>),
}

impl fmt::Display for LakestreamError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LakestreamError::Io(e) => write!(f, "{}", e),
            LakestreamError::Parse(e) => write!(f, "{}", e),
            LakestreamError::String(s) => write!(f, "{}", s),
            LakestreamError::Wrapped(e) => write!(f, "{}", e),
            #[cfg(target_arch = "wasm32")]
            LakestreamError::Js(e) => write!(
                f,
                "JsError: {}",
                e.as_string().unwrap_or_else(|| "Unknown error".to_string())
            ),
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

#[cfg(target_arch = "wasm32")]
impl From<wasm_bindgen::JsValue> for LakestreamError {
    fn from(error: wasm_bindgen::JsValue) -> Self {
        LakestreamError::Js(error)
    }
}
