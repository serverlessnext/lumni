use localencrypt::SecureStringError;

#[derive(Debug, Clone)]
pub enum FormError {
    SubmitError(String),
    ValidationError { field: String, details: String },
    LocalEncryptError(SecureStringError),
}

impl std::fmt::Display for FormError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormError::SubmitError(msg) => write!(f, "Submit error: {}", msg),
            FormError::ValidationError { field, details } => {
                write!(f, "Validation error: {} - {}", field, details)
            }
            FormError::LocalEncryptError(err) => {
                write!(f, "LocalEncrypt error: {}", err)
            }
        }
    }
}

impl From<SecureStringError> for FormError {
    fn from(err: SecureStringError) -> Self {
        FormError::LocalEncryptError(err)
    }
}

impl From<String> for FormError {
    fn from(s: String) -> Self {
        FormError::SubmitError(s)
    }
}
