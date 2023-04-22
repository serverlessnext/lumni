pub mod formatters;
pub mod time;
pub mod time_parse;

#[cfg(not(target_arch = "wasm32"))]
mod time_parse_ext;

#[cfg(target_arch = "wasm32")]
#[path = "time_parse_ext_wasm32.rs"]
mod time_parse_ext;
