use std::fmt;

// export the http client error via api::error
pub use crate::http::client::HttpClientError;

#[allow(dead_code)]
#[derive(Debug)]
pub enum LumniError {
    Request(RequestError),
    Runtime(RuntimeError),
    Application(ApplicationError, Option<String>),
    Invoke(ApplicationError, Option<String>),
    NotImplemented(String),
    Message(String),
}

#[derive(Debug, Clone)]
pub enum RequestError {
    QueryInvalid(String),
}

//#[allow(dead_code)]
#[derive(Debug)]
pub enum ApplicationError {
    InvalidUserConfiguration(String),
    Unexpected(String),
    Runtime(String),
    InvalidCredentials(String),
    ServerConfigurationError(String),
    HttpClientError(HttpClientError),
    IoError(std::io::Error),
    NotImplemented(String),
    NotReady(String),
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum RuntimeError {
    Unexpected(String),
}

impl fmt::Display for LumniError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LumniError::Request(req_err) => {
                write!(f, "RequestError: {}", req_err)
            }
            LumniError::Runtime(runtime_err) => {
                write!(f, "RuntimeError: {}", runtime_err)
            }
            LumniError::Application(app_err, Some(app_name)) => {
                write!(f, "[{}]: {}", app_name, app_err)
            }
            LumniError::Invoke(app_err, Some(app_name)) => {
                write!(f, "[{}]: {}", app_name, app_err)
            }
            LumniError::Invoke(app_err, None) => {
                write!(f, "InvokeError: {}", app_err)
            }
            LumniError::Application(app_err, None) => {
                write!(f, "ApplicationError: {}", app_err)
            }
            LumniError::NotImplemented(s) => write!(f, "NotImplemented: {}", s),
            LumniError::Message(s) => write!(f, "{}", s),
        }
    }
}

impl fmt::Display for RequestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RequestError::QueryInvalid(s) => write!(f, "QueryInvalid: {}", s),
        }
    }
}

impl From<LumniError> for ApplicationError {
    fn from(error: LumniError) -> Self {
        match error {
            LumniError::Application(app_error, None) => app_error,
            _ => {
                ApplicationError::Unexpected("Unhandled LumniError".to_string())
            }
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
            ApplicationError::InvalidUserConfiguration(s) => {
                write!(f, "InvalidUserConfiguration: {}", s)
            }
            ApplicationError::Unexpected(s) => write!(f, "Unexpected: {}", s),
            ApplicationError::Runtime(s) => write!(f, "Runtime: {}", s),
            ApplicationError::InvalidCredentials(s) => {
                write!(f, "InvalidCredentials: {}", s)
            }
            ApplicationError::ServerConfigurationError(s) => {
                write!(f, "ServerConfigurationError: {}", s)
            }
            ApplicationError::HttpClientError(e) => {
                write!(f, "HttpClientError: {}", e)
            }
            ApplicationError::IoError(e) => write!(f, "IoError: {}", e),
            ApplicationError::NotImplemented(s) => {
                write!(f, "NotImplemented: {}", s)
            }
            ApplicationError::NotReady(s) => write!(f, "NotReady: {}", s),
        }
    }
}

impl std::error::Error for ApplicationError {}

impl From<HttpClientError> for ApplicationError {
    fn from(error: HttpClientError) -> Self {
        ApplicationError::HttpClientError(error)
    }
}

impl From<std::io::Error> for ApplicationError {
    fn from(error: std::io::Error) -> Self {
        ApplicationError::IoError(error)
    }
}
