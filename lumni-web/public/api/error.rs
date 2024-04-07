use std::fmt;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Error {
    Request(RequestError),
    Runtime(RuntimeError),
    Application(ApplicationError),
}

#[derive(Debug, Clone)]
pub enum RequestError {
    QueryInvalid(String),
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum ApplicationError {
    ConfigInvalid(String),
    Unexpected(String),
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum RuntimeError {
    Unexpected(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Request(req_err) => write!(f, "Request Error: {}", req_err),
            Error::Runtime(runtime_err) => {
                write!(f, "Runtime Error: {}", runtime_err)
            }
            Error::Application(app_err) => {
                write!(f, "Application Error: {}", app_err)
            }
        }
    }
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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

impl fmt::Display for ApplicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApplicationError::ConfigInvalid(s) => {
                write!(f, "Config Invalid: {}", s)
            }
            ApplicationError::Unexpected(s) => write!(f, "Unexpected: {}", s),
        }
    }
}
