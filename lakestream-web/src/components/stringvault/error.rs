use std::fmt;

use base64::DecodeError;
use wasm_bindgen::JsValue;

pub type SecureStringResult<T> = Result<T, SecureStringError>;

#[derive(Debug)]
pub enum SecureStringError {
    JsError(JsValue),
    DecryptError(String),
    Base64Error(DecodeError),
    NoWindow,
    NoCrypto,
    NoLocalStorageData,
    PasswordNotFound(String),
    EmptyPassword,
    SaltNotStored,
    InvalidCryptoKey,
    SerdeError(serde_json::Error),
}

impl From<JsValue> for SecureStringError {
    fn from(e: JsValue) -> Self {
        SecureStringError::JsError(e)
    }
}

impl From<DecodeError> for SecureStringError {
    fn from(e: DecodeError) -> Self {
        SecureStringError::Base64Error(e)
    }
}

impl From<serde_json::Error> for SecureStringError {
    fn from(e: serde_json::Error) -> Self {
        SecureStringError::SerdeError(e)
    }
}

impl fmt::Display for SecureStringError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecureStringError::JsError(e) => {
                write!(f, "JavaScript error: {:?}", e)
            }
            SecureStringError::DecryptError(msg) => {
                write!(f, "Decryption error: {}", msg)
            }
            SecureStringError::Base64Error(e) => {
                write!(f, "Base64 decoding error: {}", e)
            }
            SecureStringError::NoWindow => write!(f, "No window found"),
            SecureStringError::NoCrypto => write!(f, "No crypto key found"),
            SecureStringError::NoLocalStorageData => {
                write!(f, "No data in local storage")
            }
            SecureStringError::PasswordNotFound(msg) => {
                write!(f, "Password not found: {}", msg)
            }
            SecureStringError::EmptyPassword => {
                write!(f, "Password is empty")
            }
            SecureStringError::SaltNotStored => {
                write!(f, "Salt not stored")
            }
            SecureStringError::InvalidCryptoKey => {
                write!(f, "Invalid crypto key")
            }
            SecureStringError::SerdeError(e) => {
                write!(f, "Serde JSON error: {}", e)
            }
        }
    }
}
