use std::fmt;

// export the http client error via api::error
pub use crate::http::client::HttpClientError;

#[allow(dead_code)]
#[derive(Debug)]
pub enum Error {
    Request(RequestError),
    Runtime(RuntimeError),
    Application(ApplicationError),
    Invoke(ApplicationError),
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
}


#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum RuntimeError {
    Unexpected(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Request(req_err) => write!(f, "RequestError: {}", req_err),
            Error::Runtime(runtime_err) => {
                write!(f, "RuntimeError: {}", runtime_err)
            }
            Error::Application(app_err) => {
                write!(f, "ApplicationError: {}", app_err)
            }
            Error::Invoke(app_err) => write!(f, "InvokeError: {}", app_err),
            Error::NotImplemented(s) => write!(f, "NotImplemented: {}", s),
            Error::Message(s) => write!(f, "{}", s),
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

impl From<ApplicationError> for Error {
    fn from(err: ApplicationError) -> Self {
        Error::Application(err)
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
            ApplicationError::HttpClientError(e) => write!(f, "HttpClientError: {}", e),
            ApplicationError::IoError(e) => write!(f, "IoError: {}", e),
            ApplicationError::NotImplemented(s) => write!(f, "NotImplemented: {}", s),
        }
    }
}

impl std::error::Error for ApplicationError {}

impl From<Box<dyn std::error::Error>> for Error {
    // any other Error type can be assumed to come from an
    // application Invoke() method
    fn from(error: Box<dyn std::error::Error>) -> Self {
        Error::Invoke(ApplicationError::Runtime(error.to_string()))
    }
}

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