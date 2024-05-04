mod responses;
mod send;
mod session;

pub use responses::ChatCompletionResponse;
pub use session::{process_prompt, process_prompt_response, ChatSession};

//use std::env;
//const OUT_DIR : &str =concat!(env!("OUT_DIR"), "/llm/prompt");
//include!(concat!(OUT_DIR, "/templates.rs"));
include!(concat!(env!("OUT_DIR"), "/llm/prompt/templates.rs"));
