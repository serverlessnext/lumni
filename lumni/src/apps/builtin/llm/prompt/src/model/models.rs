use std::collections::HashMap;
use std::error::Error;

use async_trait::async_trait;
use bytes::Bytes;
use lumni::HttpClient;
use serde::Deserialize;
use serde_json::json;
use url::Url;

use super::generic::Generic;
use super::llama3::Llama3;
use super::{LlamaServerSystemPrompt, PromptOptions, Endpoints};
use crate::external as lumni;


pub enum PromptRole {
    User,
    Assistant,
    System,
}

#[derive(Clone)]
pub enum PromptModel {
    Generic(Generic),
    Llama3(Llama3),
}

impl PromptModel {
    pub fn default() -> Result<Self, Box<dyn Error>> {
        Ok(PromptModel::Generic(Generic::new()?))
    }

    pub fn from_str(s: &str) -> Result<Self, Box<dyn Error>> {
        match s {
            "llama3" => Ok(PromptModel::Llama3(Llama3::new()?)),
            _ => Ok(PromptModel::Generic(Generic::new()?)),
        }
    }
}

impl PromptModelTrait for PromptModel {
    fn get_prompt_options(&self) -> &PromptOptions {
        match self {
            PromptModel::Generic(generic) => generic.get_prompt_options(),
            PromptModel::Llama3(llama3) => llama3.get_prompt_options(),
        }
    }

    fn get_endpoints(&self) -> &Endpoints {
        match self {
            PromptModel::Generic(generic) => generic.get_endpoints(),
            PromptModel::Llama3(llama3) => llama3.get_endpoints(),
        }
    }

    fn get_stop_tokens(&self) -> &Vec<String> {
        match self {
            PromptModel::Generic(generic) => generic.get_stop_tokens(),
            PromptModel::Llama3(llama3) => llama3.get_stop_tokens(),
        }
    }

    fn update_options_from_json(&mut self, json: &str) {
        match self {
            PromptModel::Generic(generic) => {
                generic.update_options_from_json(json)
            }
            PromptModel::Llama3(llama3) => {
                llama3.update_options_from_json(json)
            }
        }
    }

    fn set_context_size(&mut self, context_size: usize) {
        match self {
            PromptModel::Generic(generic) => {
                generic.set_context_size(context_size)
            }
            PromptModel::Llama3(llama3) => {
                llama3.set_context_size(context_size)
            }
        }
    }

    fn fmt_prompt_system(&self, instruction: Option<&str>) -> String {
        match self {
            PromptModel::Generic(generic) => {
                generic.fmt_prompt_system(instruction)
            }
            PromptModel::Llama3(llama3) => {
                llama3.fmt_prompt_system(instruction)
            }
        }
    }

    fn fmt_prompt_message(
        &self,
        prompt_role: PromptRole,
        message: &str,
    ) -> String {
        match self {
            PromptModel::Generic(generic) => {
                generic.fmt_prompt_message(prompt_role, message)
            }
            PromptModel::Llama3(llama3) => {
                llama3.fmt_prompt_message(prompt_role, message)
            }
        }
    }
}

#[async_trait]
pub trait PromptModelTrait: Send + Sync {
    fn get_prompt_options(&self) -> &PromptOptions;
    fn get_endpoints(&self) -> &Endpoints;
    fn get_stop_tokens(&self) -> &Vec<String>;
    fn update_options_from_json(&mut self, json: &str);
    fn set_context_size(&mut self, context_size: usize);

    fn fmt_prompt_system(&self, instruction: Option<&str>) -> String {
        if let Some(instruction) = instruction {
            instruction.to_string()
        } else {
            "".to_string()
        }
    }

    fn get_completion_endpoint(&self) -> &Url {
        self.get_endpoints().get_completion()
    }

    fn get_settings_endpoint(&self) -> &Url {
        self.get_endpoints().get_settings()
    }

    fn get_tokenizer_endpoint(&self) -> &Url {
        self.get_endpoints().get_tokenizer()
    }

    fn get_role_prefix(&self, prompt_role: PromptRole) -> &str {
        match prompt_role {
            PromptRole::User => "### User: ",
            PromptRole::Assistant => "### Assistant: ",
            PromptRole::System => "",
        }
    }

    fn fmt_prompt_message(
        &self,
        prompt_role: PromptRole,
        message: &str,
    ) -> String {
        let prompt_message = match prompt_role {
            PromptRole::User => self.get_role_prefix(prompt_role).to_string(),
            PromptRole::Assistant => {
                self.get_role_prefix(prompt_role).to_string()
            }
            PromptRole::System => self.get_role_prefix(prompt_role).to_string(),
        };

        if message.is_empty() {
            prompt_message // initiate new message, not yet completed
        } else {
            format!("{}{}\n", prompt_message, message) // message already completed
        }
    }

    async fn tokenizer(
        &self,
        content: &str,
        http_client: &HttpClient,
    ) -> Result<TokenResponse, Box<dyn Error>> {
        let body_content =
            serde_json::to_string(&json!({ "content": content }))?;
        let body = Bytes::copy_from_slice(body_content.as_bytes());

        let url = self.get_tokenizer_endpoint().to_string();
        let mut headers = HashMap::new();
        headers
            .insert("Content-Type".to_string(), "application/json".to_string());

        let http_response = http_client
            .post(&url, Some(&headers), None, Some(&body), None, None)
            .await
            .map_err(|e| format!("Error calling tokenizer: {}", e))?;

        let response = http_response.json::<TokenResponse>()?;
        Ok(response)
    }

    fn get_system_prompt(&self, instruction: &str) -> LlamaServerSystemPrompt {
        LlamaServerSystemPrompt::new(
            instruction.to_string(),
            self.get_role_prefix(PromptRole::User).to_string(),
            self.get_role_prefix(PromptRole::Assistant).to_string(),
        )
    }
}

#[derive(Deserialize)]
pub struct TokenResponse {
    tokens: Vec<usize>,
}

impl TokenResponse {
    pub fn get_tokens(&self) -> &Vec<usize> {
        &self.tokens
    }
}

