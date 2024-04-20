mod responses;
mod send;
mod session;

pub use responses::ChatCompletionResponse;
pub use session::ChatSession;

include!(concat!(env!("OUT_DIR"), "/llm/agent/templates.rs"));
