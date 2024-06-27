use std::error::Error;

use async_trait::async_trait;
use regex::Regex;

use super::generic::Generic;
use super::llama3::Llama3;

pub enum PromptRole {
    User,
    Assistant,
    System,
}

#[derive(Clone, Debug)]
enum PromptModel {
    Generic(Generic),
    Llama3(Llama3),
}

#[derive(Clone, Debug)]
pub struct ModelFormatter {
    model: PromptModel,
}

impl ModelFormatter {
    pub fn from_str(s: &str) -> Self {
        let model_name = s.to_lowercase();
        // infer model name from given pattern, e.g. llama-3, llama3
        // used for llama.cpp only
        // TODO: add more patterns for more popular models
        let llama3_pattern = Regex::new(r"llama-?3").unwrap();
        // add more patterns for other models
        let model = if llama3_pattern.is_match(&model_name) {
            PromptModel::Llama3(Llama3::new())
        } else {
            PromptModel::Generic(Generic::new(s))
        };
        ModelFormatter { model }
    }
}

impl ModelFormatterTrait for ModelFormatter {
    fn get_stop_tokens(&self) ->  &Vec<String> {
        match self.model {
            PromptModel::Generic(ref generic) => generic.get_stop_tokens(),
            PromptModel::Llama3(ref llama3) => llama3.get_stop_tokens(),
        }
    }

    fn fmt_prompt_system(&self, instruction: Option<&str>) -> String {
        match self.model {
            PromptModel::Generic(ref generic) => {
                generic.fmt_prompt_system(instruction)
            }
            PromptModel::Llama3(ref llama3) => {
                llama3.fmt_prompt_system(instruction)
            }
        }
    }

    fn fmt_prompt_message(&self,prompt_role:PromptRole,message: &str,) -> String {
        match self.model {
            PromptModel::Generic(ref generic) => {
                generic.fmt_prompt_message(prompt_role, message)
            }
            PromptModel::Llama3(ref llama3) => {
                llama3.fmt_prompt_message(prompt_role, message)
            }
        }
    }
}

#[async_trait]
pub trait ModelFormatterTrait: Send + Sync {
    fn get_stop_tokens(&self) -> &Vec<String>;

    fn fmt_prompt_system(&self, instruction: Option<&str>) -> String {
        if let Some(instruction) = instruction {
            instruction.to_string()
        } else {
            "".to_string()
        }
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
}
