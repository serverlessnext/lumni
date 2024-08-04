use std::error::Error;
use std::fmt;

use lumni::api::error::ApplicationError;

pub use crate::external as lumni;

#[derive(Debug)]
pub enum PromptError {
    NotReady(PromptNotReadyReason),
    ServerConfigurationError(String),
    Runtime(String),
}

#[derive(Debug)]
pub enum PromptNotReadyReason {
    NoModelSelected,
    Other(String),
}

impl fmt::Display for PromptNotReadyReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PromptNotReadyReason::NoModelSelected => {
                write!(f, "NoModelSelected")
            }
            PromptNotReadyReason::Other(msg) => {
                write!(f, "{}", msg)
            }
        }
    }
}

impl fmt::Display for PromptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PromptError::NotReady(reason) => {
                write!(f, "{}", reason)
            }
            PromptError::ServerConfigurationError(msg) => {
                write!(f, "{}", msg)
            }
            PromptError::Runtime(msg) => {
                write!(f, "{}", msg)
            }
        }
    }
}

impl Error for PromptError {}

impl From<PromptError> for ApplicationError {
    fn from(error: PromptError) -> Self {
        match error {
            PromptError::NotReady(msg) => {
                ApplicationError::NotReady(msg.to_string())
            }
            PromptError::ServerConfigurationError(msg) => {
                ApplicationError::ServerConfigurationError(msg)
            }
            PromptError::Runtime(msg) => ApplicationError::Runtime(msg),
        }
    }
}
