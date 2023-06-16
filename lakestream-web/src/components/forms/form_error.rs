#[derive(Debug, Clone)]
pub enum FormError {
    SubmitError(String),
}

impl std::fmt::Display for FormError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormError::SubmitError(msg) => write!(f, "Submit error: {}", msg),
        }
    }
}
