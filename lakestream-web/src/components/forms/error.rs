use std::error::Error;
impl std::error::Error for FormError {}

#[derive(Debug, Clone)]
pub enum FormError {
    ValidationError(String),
    SubmitError(String),
    // add more variants as needed
}

impl std::fmt::Display for FormError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormError::ValidationError(msg) => {
                write!(f, "Validation error: {}", msg)
            }
            FormError::SubmitError(msg) => write!(f, "Submit error: {}", msg),
            // add more match arms as needed
        }
    }
}
