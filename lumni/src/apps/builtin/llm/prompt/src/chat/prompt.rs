use serde::{Deserialize, Serialize};

use super::exchange::ChatExchange;

#[derive(Debug, Serialize, Deserialize)]
pub struct Prompt {
    name: String,
    system_prompt: Option<String>,
    prompt_template: Option<String>,
    exchanges: Option<Vec<ChatExchange>>,
}

impl Prompt {
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn system_prompt(&self) -> Option<&str> {
        self.system_prompt.as_deref()
    }

    pub fn prompt_template(&self) -> Option<&str> {
        self.prompt_template.as_deref()
    }

    pub fn exchanges(&self) -> Option<&Vec<ChatExchange>> {
        self.exchanges.as_ref()
    }
}
