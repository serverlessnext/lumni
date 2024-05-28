mod generic;
mod llama3;

pub use generic::Generic;
pub use llama3::Llama3;

pub enum Models {
    Generic,
    Llama3,
}

impl Models {
    pub fn from_str(s: &str) -> Self {
        match s {
            "llama3" => Models::Llama3,
            _ => Models::Generic,
        }
    }

    pub fn role_name_user(&self) -> String {
        match self {
            Models::Generic => Generic.role_name_user(),
            Models::Llama3 => Llama3.role_name_user(),
        }
    }

    pub fn role_name_system(&self) -> String {
        match self {
            Models::Generic => Generic.role_name_system(),
            Models::Llama3 => Llama3.role_name_system(),
        }
    }

    pub fn role_name_assistant(&self) -> String {
        match self {
            Models::Generic => Generic.role_name_assistant(),
            Models::Llama3 => Llama3.role_name_assistant(),
        }
    }

    pub fn fmt_prompt_system(&self, instruction: Option<&str>) -> String {
        match self {
            Models::Generic => Generic.fmt_prompt_system(instruction),
            Models::Llama3 => Llama3.fmt_prompt_system(instruction),
        }
    }

    pub fn fmt_prompt_message(&self, role: &str, message: &str) -> String {
        match self {
            Models::Generic => Generic.fmt_prompt_message(role, message),
            Models::Llama3 => Llama3.fmt_prompt_message(role, message),
        }
    }
}

pub trait PromptModel {
    fn fmt_prompt_system(&self, instruction: Option<&str>) -> String;
    fn fmt_prompt_message(&self, role: &str, message: &str) -> String;

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


