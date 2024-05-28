use super::PromptModel;

pub struct Generic;

impl PromptModel for Generic {
    fn fmt_prompt_system(&self, instruction: Option<&str>) -> String {
        instruction.unwrap_or("").to_string()   // keep instruction as-is
    }

    fn fmt_prompt_message(&self, role: &str, message: &str) -> String {
        match role {
            "user" => format!("### Human: {}", message),
            "assistant" => format!("### Assistant: {}", message),
            _ => format!("### {}: {}", role, message), // Default case for any other role
        }
    }
}
