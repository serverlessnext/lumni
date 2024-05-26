use super::PromptModel;

pub struct Llama3;

impl PromptModel for Llama3 {
    fn fmt_prompt_start(&self, instruction: Option<&str>) -> String {
        if let Some(instruction) = instruction {
            return format!(
                "<|begin_of_text|><|start_header_id|>system<|end_header_id|>{}<|eot_id|>\n",
                instruction
            ).to_string()
        } else {
            return "<|begin_of_text|>".to_string()
        }
    }
}
