#[cfg(not(target_arch = "wasm32"))]
pub mod requests;
mod client;
pub use client::{HttpClient, HttpClientError};

#[cfg(target_arch = "wasm32")]
#[path = "requests_wasm32.rs"]
pub mod requests;
