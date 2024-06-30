use lumni::{HttpClientError, HttpClientErrorHandler, HttpClientResponse};

pub use crate::external as lumni;

pub struct AWSErrorHandler;

impl HttpClientErrorHandler for AWSErrorHandler {
    fn handle_error(
        &self,
        response: HttpClientResponse,
        canonical_reason: String,
    ) -> HttpClientError {
        if response.status_code() == 403 {
            if let Some(value) = response.headers().get("x-amzn-errortype") {
                if let Ok(err_type) = value.to_str() {
                    if err_type.starts_with("ExpiredTokenException") {
                        return HttpClientError::HttpError(
                            403,
                            "ExpiredToken".to_string(),
                        );
                    }
                }
            }
        }
        // Fallback if no special handling is needed
        HttpClientError::HttpError(response.status_code(), canonical_reason)
    }
}
