pub mod formatters;
pub mod time;

#[cfg(not(target_arch = "wasm32"))]
mod timeparse;

#[cfg(target_arch = "wasm32")]
#[path = "timeparse_wasm32.rs"]
mod timeparse;
// pub mod requests_wasm;
