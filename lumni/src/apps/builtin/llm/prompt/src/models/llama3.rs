use super::PromptModel;

pub struct Llama3;

impl PromptModel for Llama3 {
    fn fmt_prompt_system(&self, instruction: Option<&str>) -> String {
        if let Some(instruction) = instruction {
            return format!(
                "<|begin_of_text|>{}",
                self.fmt_prompt_message("system", instruction)
            ).to_string()
        } else {
            return "<|begin_of_text|>".to_string()
        }
    }

    fn fmt_prompt_message(&self, role: &str, message: &str) -> String {
        format!(
            "<|start_header_id|>{}<|end_header_id|>{}\n<|eot_id|>\n",
            role, message
        )
    }
}
