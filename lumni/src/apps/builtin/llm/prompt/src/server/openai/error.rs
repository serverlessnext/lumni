use lumni::{HttpClientError, HttpClientErrorHandler, HttpClientResponse};

pub use crate::external as lumni;

pub struct OpenAIErrorHandler;

impl HttpClientErrorHandler for OpenAIErrorHandler {
    fn handle_error(
        &self,
        response: HttpClientResponse,
        canonical_reason: String,
    ) -> HttpClientError {
        // Fallback if no special handling is needed
        HttpClientError::HttpError(response.status_code(), canonical_reason)
    }
}
