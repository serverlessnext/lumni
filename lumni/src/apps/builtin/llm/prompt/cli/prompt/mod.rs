mod responses;
mod send;
mod session;

pub use responses::ChatCompletionResponse;
pub use session::{process_prompt, process_prompt_response, ChatSession};

include!(concat!(env!("OUT_DIR"), "/llm/prompt/templates.rs"));
