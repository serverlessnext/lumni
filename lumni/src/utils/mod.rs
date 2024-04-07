pub mod formatters;
pub mod string_replace;
pub mod time;
pub mod time_parse;
mod uri_parse;
pub use uri_parse::{ParsedUri, UriScheme};

#[cfg(not(target_arch = "wasm32"))]
mod time_parse_ext;

#[cfg(target_arch = "wasm32")]
#[path = "time_parse_ext_wasm32.rs"]
mod time_parse_ext;
