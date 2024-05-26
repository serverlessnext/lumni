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

    pub fn fmt_prompt_start(&self, instruction: Option<&str>) -> String {
        match self {
            Models::Llama3 => Llama3.fmt_prompt_start(instruction),
        }
    }
}

pub trait PromptModel {
    fn fmt_prompt_start(&self, instruction: Option<&str>) -> String;
}