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
use super::{ChatCompletionOptions, PromptOptions};
use crate::external as lumni;

pub const DEFAULT_TOKENIZER_ENDPOINT: &str = "http://localhost:8080/tokenize";
pub const DEFAULT_COMPLETION_ENDPOINT: &str =
    "http://localhost:8080/completion";

pub enum Models {
    Generic(Generic),
    Llama3(Llama3),
}

impl Models {
    pub fn default() -> Result<Self, Box<dyn Error>> {
        Ok(Models::Generic(Generic::new()?))
    }

    pub fn from_str(s: &str) -> Result<Self, Box<dyn Error>> {
        match s {
            "llama3" => Ok(Models::Llama3(Llama3::new()?)),
            _ => Ok(Models::Generic(Generic::new()?)),
        }
    }
}

impl PromptModel for Models {
    fn get_prompt_options(&self) -> &PromptOptions {
        match self {
            Models::Generic(generic) => generic.get_prompt_options(),
            Models::Llama3(llama3) => llama3.get_prompt_options(),
        }
    }

    fn get_completion_options(&self) -> &ChatCompletionOptions {
        match self {
            Models::Generic(generic) => generic.get_completion_options(),
            Models::Llama3(llama3) => llama3.get_completion_options(),
        }
    }

    fn get_endpoints(&self) -> &Endpoints {
        match self {
            Models::Generic(generic) => generic.get_endpoints(),
            Models::Llama3(llama3) => llama3.get_endpoints(),
        }
    }

    fn update_options_from_json(&mut self, json: &str) {
        match self {
            Models::Generic(generic) => generic.update_options_from_json(json),
            Models::Llama3(llama3) => llama3.update_options_from_json(json),
        }
    }

    fn set_n_keep(&mut self, n_keep: usize) {
        match self {
            Models::Generic(generic) => generic.set_n_keep(n_keep),
            Models::Llama3(llama3) => llama3.set_n_keep(n_keep),
        }
    }

    fn role_name_user(&self) -> String {
        match self {
            Models::Generic(generic) => generic.role_name_user(),
            Models::Llama3(llama3) => llama3.role_name_user(),
        }
    }

    fn role_name_assistant(&self) -> String {
        match self {
            Models::Generic(generic) => generic.role_name_assistant(),
            Models::Llama3(llama3) => llama3.role_name_assistant(),
        }
    }

    fn fmt_prompt_system(&self, instruction: Option<&str>) -> String {
        match self {
            Models::Generic(generic) => generic.fmt_prompt_system(instruction),
            Models::Llama3(llama3) => llama3.fmt_prompt_system(instruction),
        }
    }

    fn fmt_prompt_message(&self, role: &str, message: &str) -> String {
        match self {
            Models::Generic(generic) => {
                generic.fmt_prompt_message(role, message)
            }
            Models::Llama3(llama3) => llama3.fmt_prompt_message(role, message),
        }
    }
}

#[async_trait]
pub trait PromptModel: Send + Sync {
    fn get_prompt_options(&self) -> &PromptOptions;
    fn get_completion_options(&self) -> &ChatCompletionOptions;
    fn get_endpoints(&self) -> &Endpoints;

    fn update_options_from_json(&mut self, json: &str);
    fn set_n_keep(&mut self, n_keep: usize);
    fn fmt_prompt_system(&self, instruction: Option<&str>) -> String;

    fn get_completion_endpoint(&self) -> &Url {
        self.get_endpoints().get_completion()
    }

    fn get_tokenizer_endpoint(&self) -> &Url {
        self.get_endpoints().get_tokenizer()
    }

    fn role_name_user(&self) -> String {
        "User".to_string()
    }

    fn role_name_assistant(&self) -> String {
        "Assistant".to_string()
    }

    fn fmt_prompt_message(&self, role: &str, message: &str) -> String {
        let prompt_message = match role {
            "User" => format!("### Human: {}", message),
            "Assistant" => format!("### Assistant: {}", message),
            _ => format!("### {}: {}", role, message), // Default case for any other role
        };
        if message.is_empty() {
            prompt_message // initiate new message, not yet completed
        } else {
            format!("{}\n", prompt_message) // message already completed
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

pub struct Endpoints {
    completion: Url,
    tokenizer: Url,
}

impl Endpoints {
    pub fn default() -> Result<Self, Box<dyn Error>> {
        let completion = Url::parse(DEFAULT_COMPLETION_ENDPOINT)?;
        let tokenizer = Url::parse(DEFAULT_TOKENIZER_ENDPOINT)?;

        Ok(Endpoints {
            completion,
            tokenizer,
        })
    }

    pub fn get_completion(&self) -> &Url {
        &self.completion
    }

    pub fn get_tokenizer(&self) -> &Url {
        &self.tokenizer
    }
}
