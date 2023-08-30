use std::fmt;

#[derive(Debug, Clone)]
pub enum Error {
    Request(RequestError),
    Runtime(RuntimeError),
}

#[derive(Debug, Clone)]
pub enum RequestError {
    ConfigInvalid(String),
    QueryInvalid(String),
}

#[derive(Debug, Clone)]
pub enum RuntimeError {
    Unexpected(String),
}


impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Request(req_err) => write!(f, "Request Error: {}", req_err),
            Error::Runtime(runtime_err) => write!(f, "Runtime Error: {}", runtime_err),
        }
    }
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestError::ConfigInvalid(s) => write!(f, "Config Invalid: {}", s),
            RequestError::QueryInvalid(s) => write!(f, "Query Invalid: {}", s),
        }
    }
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::Unexpected(s) => write!(f, "Unexpected: {}", s),
        }
    }
}
