use std::error::Error;

mod prompt;
mod responses;
mod send;
mod session;

use prompt::Prompt;
pub use responses::ChatCompletionResponse;
pub use session::{process_prompt, process_prompt_response, ChatSession};

pub use super::model::{
    ChatCompletionOptions, LlamaServerSystemPrompt, PromptModel,
    PromptModelTrait, PromptRole, TokenResponse,
};

// gets PERSONAS from the generated code
include!(concat!(env!("OUT_DIR"), "/llm/prompt/templates.rs"));

pub fn list_assistants() -> Result<Vec<String>, Box<dyn Error>> {
    let prompts: Vec<Prompt> = serde_yaml::from_str(PERSONAS)?;
    let assistants: Vec<String> =
        prompts.iter().map(|p| p.name().to_owned()).collect();
    Ok(assistants)
}
