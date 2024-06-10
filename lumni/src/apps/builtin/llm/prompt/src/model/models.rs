use std::error::Error;

use async_trait::async_trait;

use super::generic::Generic;
use super::llama3::Llama3;

pub enum PromptRole {
    User,
    Assistant,
    System,
}

#[derive(Clone, Debug)]
pub enum PromptModel {
    Generic(Generic),
    Llama3(Llama3),
}

impl PromptModel {
    pub fn default() -> Result<Self, Box<dyn Error>> {
        Ok(PromptModel::Generic(Generic::new("auto")?))
    }

    pub fn from_str(s: &str) -> Result<Self, Box<dyn Error>> {
        match s {
            "llama3" => Ok(PromptModel::Llama3(Llama3::new()?)),
            // fallback to generic model
            _ => Ok(PromptModel::Generic(Generic::new(s)?)),
        }
    }
}

impl PromptModelTrait for PromptModel {
    fn get_model_data(&self) -> &ModelData {
        match self {
            PromptModel::Generic(generic) => generic.get_model_data(),
            PromptModel::Llama3(llama3) => llama3.get_model_data(),
        }
    }

    fn get_stop_tokens(&self) -> &Vec<String> {
        match self {
            PromptModel::Generic(generic) => generic.get_stop_tokens(),
            PromptModel::Llama3(llama3) => llama3.get_stop_tokens(),
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
    fn get_model_data(&self) -> &ModelData;
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

#[derive(Clone, Debug)]
pub struct ModelData {
    name: String,
}

impl ModelData {
    pub fn new(name: &str) -> Self {
        ModelData {
            name: name.to_string(),
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }
}
