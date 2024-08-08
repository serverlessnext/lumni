use std::error::Error;
use std::fmt;

use base64::engine::general_purpose;
use base64::Engine as _;
use ring::aead;
use ring::rand::{SecureRandom, SystemRandom};
use rsa::{Pkcs1v15Encrypt, RsaPrivateKey, RsaPublicKey};
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
    EncryptionError(EncryptionError),
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
            ApplicationError::EncryptionError(e) => {
                write!(f, "EncryptionError: {}", e)
            }
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

#[derive(Debug)]
pub enum EncryptionError {
    RsaError(rsa::Error),
    RingError(String),
    Base64Error(base64::DecodeError),
    Utf8Error(std::string::FromUtf8Error),
    SpkiError(rsa::pkcs8::spki::Error),
    Pkcs8Error(rsa::pkcs8::Error),
    Other(Box<dyn Error + Send + Sync>),
}

impl fmt::Display for EncryptionError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EncryptionError::RsaError(e) => write!(f, "RSA error: {}", e),
            EncryptionError::RingError(e) => {
                write!(f, "Ring encryption error: {}", e)
            }
            EncryptionError::Base64Error(e) => {
                write!(f, "Base64 decoding error: {}", e)
            }
            EncryptionError::Utf8Error(e) => {
                write!(f, "UTF-8 conversion error: {}", e)
            }
            EncryptionError::SpkiError(e) => write!(f, "SPKI error: {}", e),
            EncryptionError::Pkcs8Error(e) => write!(f, "PKCS8 error: {}", e),
            EncryptionError::Other(e) => write!(f, "Other error: {}", e),
        }
    }
}

impl Error for EncryptionError {}

impl From<rsa::Error> for EncryptionError {
    fn from(err: rsa::Error) -> EncryptionError {
        EncryptionError::RsaError(err)
    }
}

impl From<ring::error::Unspecified> for EncryptionError {
    fn from(_: ring::error::Unspecified) -> EncryptionError {
        EncryptionError::RingError("Unspecified Ring error".to_string())
    }
}

impl From<base64::DecodeError> for EncryptionError {
    fn from(err: base64::DecodeError) -> EncryptionError {
        EncryptionError::Base64Error(err)
    }
}

impl From<std::string::FromUtf8Error> for EncryptionError {
    fn from(err: std::string::FromUtf8Error) -> EncryptionError {
        EncryptionError::Utf8Error(err)
    }
}

impl From<rsa::pkcs8::spki::Error> for EncryptionError {
    fn from(err: rsa::pkcs8::spki::Error) -> EncryptionError {
        EncryptionError::SpkiError(err)
    }
}

impl From<rsa::pkcs8::Error> for EncryptionError {
    fn from(err: rsa::pkcs8::Error) -> EncryptionError {
        EncryptionError::Pkcs8Error(err)
    }
}

impl From<Box<dyn Error + Send + Sync>> for EncryptionError {
    fn from(err: Box<dyn Error + Send + Sync>) -> EncryptionError {
        EncryptionError::Other(err)
    }
}

// Implement From<EncryptionError> for ApplicationError
impl From<EncryptionError> for ApplicationError {
    fn from(err: EncryptionError) -> ApplicationError {
        ApplicationError::CustomError(Box::new(err))
    }
}
