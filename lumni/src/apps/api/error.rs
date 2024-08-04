use std::error::Error;
use std::fmt;

use rusqlite::Error as SqliteError;
use tokio::task::JoinError;

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
    Any(String),
}

#[derive(Debug, Clone)]
pub enum RequestError {
    QueryInvalid(String),
}

#[derive(Debug)]
pub enum ApplicationError {
    InvalidUserConfiguration(String),
    Unexpected(String),
    Runtime(String),
    ChannelError(String),
    InvalidCredentials(String),
    InvalidInput(String),
    NotFound(String),
    ServerConfigurationError(String),
    HttpClientError(HttpClientError),
    IoError(std::io::Error),
    DatabaseError(String),
    NotImplemented(String),
    NotReady(String),
    CustomError(Box<dyn Error + Send + Sync>),
}

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
            LumniError::Any(s) => write!(f, "{}", s),
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
            ApplicationError::ChannelError(s) => {
                write!(f, "ChannelError: {}", s)
            }
            ApplicationError::InvalidCredentials(s) => {
                write!(f, "InvalidCredentials: {}", s)
            }
            ApplicationError::InvalidInput(s) => {
                write!(f, "InvalidInput: {}", s)
            }
            ApplicationError::NotFound(s) => write!(f, "NotFound: {}", s),
            ApplicationError::ServerConfigurationError(s) => {
                write!(f, "ServerConfigurationError: {}", s)
            }
            ApplicationError::HttpClientError(e) => {
                write!(f, "HttpClientError: {}", e)
            }
            ApplicationError::IoError(e) => write!(f, "IoError: {}", e),
            ApplicationError::DatabaseError(s) => {
                write!(f, "DatabaseError: {}", s)
            }
            ApplicationError::NotImplemented(s) => {
                write!(f, "NotImplemented: {}", s)
            }
            ApplicationError::NotReady(s) => write!(f, "NotReady: {}", s),
            ApplicationError::CustomError(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for ApplicationError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ApplicationError::CustomError(e) => Some(e.as_ref()),
            // For other variants, we use the default behavior (returning None)
            _ => None,
        }
    }
}

impl From<HttpClientError> for ApplicationError {
    fn from(error: HttpClientError) -> Self {
        match error {
            HttpClientError::ConnectionError(e) => {
                ApplicationError::NotReady(e.to_string())
            }
            _ => ApplicationError::HttpClientError(error),
        }
    }
}

impl From<std::io::Error> for ApplicationError {
    fn from(error: std::io::Error) -> Self {
        ApplicationError::IoError(error)
    }
}

impl From<SqliteError> for ApplicationError {
    fn from(error: SqliteError) -> Self {
        ApplicationError::DatabaseError(format!(
            "Database operation failed: {}",
            error
        ))
    }
}

impl From<serde_json::Error> for ApplicationError {
    fn from(error: serde_json::Error) -> Self {
        ApplicationError::InvalidInput(format!("Invalid JSON: {}", error))
    }
}

impl From<anyhow::Error> for ApplicationError {
    fn from(err: anyhow::Error) -> Self {
        ApplicationError::Runtime(format!("Runtime error: {}", err))
    }
}

impl From<JoinError> for ApplicationError {
    fn from(error: JoinError) -> Self {
        ApplicationError::Runtime(format!("Task join error: {}", error))
    }
}

impl From<&str> for LumniError {
    fn from(error: &str) -> Self {
        LumniError::Any(error.to_owned())
    }
}

impl From<std::string::String> for LumniError {
    fn from(error: std::string::String) -> Self {
        LumniError::Any(error.to_owned())
    }
}
