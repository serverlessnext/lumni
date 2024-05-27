mod llama3;

pub use llama3::Llama3;

pub enum Models {
    Llama3,
}

impl Models {
    pub fn from_str(s: &str) -> Self {
        match s {
            "llama3" => Models::Llama3,
            _ => panic!("Invalid model: {}", s),
        }
    }

    pub fn role_name_user(&self) -> String {
        match self {
            Models::Llama3 => Llama3.role_name_user(),
        }
    }

    pub fn role_name_system(&self) -> String {
        match self {
            Models::Llama3 => Llama3.role_name_system(),
        }
    }

    pub fn role_name_assistant(&self) -> String {
        match self {
            Models::Llama3 => Llama3.role_name_assistant(),
        }
    }

    pub fn fmt_prompt_start(&self, instruction: Option<&str>) -> String {
        match self {
            Models::Llama3 => Llama3.fmt_prompt_start(instruction),
        }
    }

    pub fn fmt_prompt_message(&self, role: &str, message: &str) -> String {
        match self {
            Models::Llama3 => Llama3.fmt_prompt_message(role, message),
        }
    }
}

pub trait PromptModel {
    fn role_name_user(&self) -> String;
    fn role_name_system(&self) -> String;
    fn role_name_assistant(&self) -> String;
    fn fmt_prompt_start(&self, instruction: Option<&str>) -> String;
    fn fmt_prompt_message(&self, role: &str, message: &str) -> String;
}