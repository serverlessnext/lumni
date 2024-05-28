mod generic;
mod llama3;

pub use super::chat::ChatOptions;
pub use generic::Generic;
pub use llama3::Llama3;

pub enum Models {
    Generic(Generic),
    Llama3(Llama3),
}

impl Models {
    pub fn default() -> Self {
        Models::Generic(Generic::new())
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "llama3" => Models::Llama3(Llama3::new()),
            _ => Models::Generic(Generic::new()),
        }
    }
}

impl PromptModel for Models {
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

    fn chat_options(&self) -> &ChatOptions {
        match self {
            Models::Generic(generic) => generic.chat_options(),
            Models::Llama3(llama3) => llama3.chat_options(),
        }
    }

    fn role_name_user(&self) -> String {
        match self {
            Models::Generic(generic) => generic.role_name_user(),
            Models::Llama3(llama3) => llama3.role_name_user(),
        }
    }

    fn role_name_system(&self) -> String {
        match self {
            Models::Generic(generic) => generic.role_name_system(),
            Models::Llama3(llama3) => llama3.role_name_system(),
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
            Models::Generic(generic) => generic.fmt_prompt_message(role, message),
            Models::Llama3(llama3) => llama3.fmt_prompt_message(role, message),
        }
    }
}

pub trait PromptModel {
    fn fmt_prompt_system(&self, instruction: Option<&str>) -> String;
    fn fmt_prompt_message(&self, role: &str, message: &str) -> String;

    fn update_options_from_json(&mut self, json: &str);
    fn chat_options(&self) -> &ChatOptions;
    fn set_n_keep(&mut self, n_keep: usize);

    fn role_name_user(&self) -> String {
        "user".to_string()
    }

    fn role_name_system(&self) -> String {
        "system".to_string()
    }

    fn role_name_assistant(&self) -> String {
        "assistant".to_string()
    }
}


